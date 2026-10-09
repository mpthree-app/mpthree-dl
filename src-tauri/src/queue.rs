//! Download queue: concurrency-limited yt-dlp jobs with progress events,
//! cancel/retry, and post-download tagging.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::{mpsc, watch};

use crate::{catalog, library, settings::Settings, tagger, tools, ytdlp, AppState};

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(default)]
pub struct Item {
    /// Direct source URL. Empty for catalog tracks: a source is searched at download time.
    pub url: Option<String>,
    /// True when the tags below are authoritative and written after download.
    pub catalog: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub track_no: u32,
    pub track_total: u32,
    pub disc_no: u32,
    pub disc_total: u32,
    pub year: String,
    pub genre: String,
    pub artwork: String,
    pub duration_ms: u64,
    pub subdir: Option<String>,
    pub index: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JobView {
    id: u64,
    title: String,
    artist: String,
    album: String,
    state: String,
    progress: f64,
    speed: String,
    eta: String,
    error: String,
    note: String,
    path: String,
    quality: String,
}

pub struct Entry {
    view: JobView,
    item: Item,
    cancel: Option<watch::Sender<bool>>,
}

#[derive(Default)]
pub struct Queue {
    jobs: Vec<Entry>,
    next_id: u64,
    running: usize,
}

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    next_id: u64,
    jobs: Vec<(JobView, Item)>,
}

fn saved_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|p| p.join("queue.json"))
}

/// Writes the queue to disk so it survives closing the app.
fn persist(app: &AppHandle) {
    let saved = {
        let st = app.state::<AppState>();
        let q = st.queue.lock().unwrap();
        Saved { next_id: q.next_id, jobs: q.jobs.iter().map(|e| (e.view.clone(), e.item.clone())).collect() }
    };
    let Some(p) = saved_path(app) else { return };
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(t) = serde_json::to_string(&saved) {
        let tmp = p.with_extension("tmp");
        if std::fs::write(&tmp, t).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }
}

/// Restores the previous session's queue; unfinished jobs go back to "queued".
pub fn restore(app: &AppHandle) {
    let Some(saved) = saved_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<Saved>(&t).ok())
    else {
        return;
    };
    let mut resumed = 0;
    {
        let st = app.state::<AppState>();
        let mut q = st.queue.lock().unwrap();
        q.next_id = saved.next_id;
        for (mut view, item) in saved.jobs {
            if !matches!(view.state.as_str(), "done" | "error" | "cancelled") {
                view.state = "queued".into();
                view.progress = 0.0;
                view.speed.clear();
                view.eta.clear();
                view.note = "Resumed after restart".into();
                resumed += 1;
            }
            q.jobs.push(Entry { view, item, cancel: None });
        }
    }
    if resumed > 0 {
        pump(app);
    }
}

fn update(app: &AppHandle, id: u64, f: impl FnOnce(&mut JobView)) {
    let st = app.state::<AppState>();
    let view = {
        let mut q = st.queue.lock().unwrap();
        match q.jobs.iter_mut().find(|e| e.view.id == id) {
            Some(e) => {
                f(&mut e.view);
                e.view.clone()
            }
            None => return,
        }
    };
    let _ = app.emit("job-update", &view);
}

pub fn add_items(app: &AppHandle, items: Vec<Item>, quality: &str) -> usize {
    let st = app.state::<AppState>();
    let mut added = vec![];
    {
        let mut q = st.queue.lock().unwrap();
        for it in items {
            let dup = q.jobs.iter().any(|e| {
                !matches!(e.view.state.as_str(), "error" | "cancelled")
                    && e.view.quality == quality
                    && if it.url.is_some() {
                        e.item.url == it.url
                    } else {
                        e.item.title == it.title && e.item.artist == it.artist && e.item.album == it.album
                    }
            });
            if dup {
                continue;
            }
            q.next_id += 1;
            let view = JobView {
                id: q.next_id,
                title: if it.title.is_empty() {
                    ytdlp::slug_title(it.url.as_deref().unwrap_or(""))
                } else {
                    it.title.clone()
                },
                artist: it.artist.clone(),
                album: it.album.clone(),
                state: "queued".into(),
                progress: 0.0,
                speed: String::new(),
                eta: String::new(),
                error: String::new(),
                note: String::new(),
                path: String::new(),
                quality: quality.into(),
            };
            added.push(view.clone());
            q.jobs.push(Entry { view, item: it, cancel: None });
        }
    }
    for v in &added {
        let _ = app.emit("job-update", v);
    }
    persist(app);
    pump(app);
    added.len()
}

pub fn pump(app: &AppHandle) {
    let st = app.state::<AppState>();
    let max = st.settings.lock().unwrap().concurrency.clamp(1, 12);
    loop {
        let started = {
            let mut q = st.queue.lock().unwrap();
            if q.running >= max {
                None
            } else if let Some(i) = q.jobs.iter().position(|e| e.view.state == "queued") {
                let (tx, rx) = watch::channel(false);
                q.running += 1;
                let e = &mut q.jobs[i];
                e.cancel = Some(tx);
                e.view.state = if e.item.url.is_some() { "downloading" } else { "resolving" }.into();
                e.view.error.clear();
                e.view.note.clear();
                e.view.progress = 0.0;
                Some((e.view.clone(), rx))
            } else {
                None
            }
        };
        let Some((view, rx)) = started else { break };
        let _ = app.emit("job-update", &view);
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            let id = view.id;
            let max_retries = app2.state::<AppState>().settings.lock().unwrap().retries;
            let mut rx = rx;
            let mut tried: Vec<String> = vec![];
            let mut attempt = 0u32;
            let res = loop {
                match run_job(&app2, id, rx.clone(), &mut tried).await {
                    Err(e) if attempt < max_retries && retryable(&e) => {
                        attempt += 1;
                        let msg = format!("Retry {attempt}/{max_retries} after: {e}");
                        update(&app2, id, |v| {
                            v.note = msg;
                            v.progress = 0.0;
                            v.speed.clear();
                            v.eta.clear();
                        });
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_secs(3 * attempt as u64)) => {}
                            _ = cancelled(&mut rx) => break Err("cancelled".to_string()),
                        }
                    }
                    other => break other,
                }
            };
            update(&app2, id, |v| {
                v.speed.clear();
                v.eta.clear();
                match res {
                    Ok((path, note)) => {
                        v.state = "done".into();
                        v.progress = 1.0;
                        v.path = path;
                        v.note = note;
                    }
                    Err(e) if e == "cancelled" => v.state = "cancelled".into(),
                    Err(e) => {
                        v.state = "error".into();
                        v.error = e;
                    }
                }
            });
            {
                let st = app2.state::<AppState>();
                let mut q = st.queue.lock().unwrap();
                q.running = q.running.saturating_sub(1);
                if let Some(e) = q.jobs.iter_mut().find(|e| e.view.id == id) {
                    e.cancel = None;
                }
            }
            persist(&app2);
            pump(&app2);
        });
    }
}

async fn cancelled(rx: &mut watch::Receiver<bool>) {
    loop {
        if *rx.borrow() {
            return;
        }
        if rx.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// Errors that will not go away by trying again.
fn retryable(e: &str) -> bool {
    const PERMANENT: [&str; 10] = [
        "cancelled", "yt-dlp not found", "Unsupported URL", "cannot create", "No matching source",
        "Private video", "members-only", "has been removed", "HTTP Error 404", "HTTP Error 410",
    ];
    !PERMANENT.iter().any(|p| e.contains(p))
}

fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let t = cleaned.trim().trim_end_matches('.').trim();
    let t: String = t.chars().take(110).collect();
    if t.is_empty() { "_".into() } else { t.trim_end().to_string() }
}

const TITLE_TOKEN: &str = "@@TITLE@@";

/// Target path without extension.
fn out_stem(s: &Settings, it: &Item) -> PathBuf {
    let mut p = PathBuf::from(&s.output_dir);
    let stem;
    if it.catalog && it.subdir.as_ref().map_or(false, |x| !x.is_empty()) {
        // Imported playlist track: playlist folder, "NN - Artist - Title".
        if s.organize {
            p.push(sanitize(it.subdir.as_deref().unwrap_or("")));
        }
        let base = format!("{} - {}", it.artist, it.title);
        stem = match it.index {
            Some(i) => format!("{i:02} - {base}"),
            None => base,
        };
    } else if it.catalog {
        if s.organize {
            let aa = if it.album_artist.is_empty() { &it.artist } else { &it.album_artist };
            p.push(sanitize(aa));
            if !it.album.is_empty() {
                p.push(sanitize(&if it.year.is_empty() {
                    it.album.clone()
                } else {
                    format!("{} ({})", it.album, it.year)
                }));
            }
        }
        stem = if it.track_no > 0 {
            let disc = if it.disc_total > 1 { format!("{}-", it.disc_no) } else { String::new() };
            format!("{disc}{:02} - {}", it.track_no, it.title)
        } else if s.organize {
            it.title.clone()
        } else {
            format!("{} - {}", it.artist, it.title)
        };
    } else {
        if s.organize {
            if let Some(sub) = it.subdir.as_ref().filter(|x| !x.is_empty()) {
                p.push(sanitize(sub));
            }
        }
        // An empty title is left to yt-dlp, which fills in the real one.
        let title = if it.title.is_empty() { TITLE_TOKEN.to_string() } else { it.title.clone() };
        stem = match it.index {
            Some(i) => format!("{i:02} - {title}"),
            None => title,
        };
    }
    p.push(sanitize(&stem));
    p
}

fn quality_args(q: &str) -> Vec<&'static str> {
    match q {
        "flac" => vec!["-f", "bestaudio/best", "-x", "--audio-format", "flac"],
        "wav" => vec!["-f", "bestaudio/best", "-x", "--audio-format", "wav"],
        "mp3_320" => vec!["-f", "bestaudio/best", "-x", "--audio-format", "mp3", "--audio-quality", "320K"],
        "mp3_v0" => vec!["-f", "bestaudio/best", "-x", "--audio-format", "mp3", "--audio-quality", "0"],
        "mp3_128" => vec!["-f", "bestaudio/best", "-x", "--audio-format", "mp3", "--audio-quality", "128K"],
        "m4a" => vec!["-f", "bestaudio[ext=m4a]/bestaudio/best", "-x", "--audio-format", "m4a"],
        "opus" => vec!["-f", "bestaudio[acodec=opus]/bestaudio/best", "-x", "--audio-format", "opus"],
        _ => vec!["-f", "bestaudio/best", "-x", "--audio-format", "best"],
    }
}

fn fmt_speed(bps: f64) -> String {
    if bps >= 1_048_576.0 {
        format!("{:.1} MB/s", bps / 1_048_576.0)
    } else {
        format!("{:.0} KB/s", bps / 1024.0)
    }
}

fn fmt_eta(s: f64) -> String {
    let s = s as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

fn pump_lines<R: AsyncRead + Unpin + Send + 'static>(r: R, tx: mpsc::UnboundedSender<String>) {
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(r).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            if tx.send(l).is_err() {
                break;
            }
        }
    });
}

fn find_output(stem: &PathBuf) -> Option<PathBuf> {
    let dir = stem.parent()?;
    let name = stem.file_name()?.to_string_lossy().to_string();
    std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
        p.file_stem().map_or(false, |s| s.to_string_lossy() == name)
            && p.extension().map_or(false, |x| {
                matches!(
                    x.to_string_lossy().to_lowercase().as_str(),
                    "mp3" | "flac" | "m4a" | "opus" | "ogg" | "wav" | "aac" | "webm" | "mka"
                )
            })
    })
}

async fn run_job(
    app: &AppHandle,
    id: u64,
    mut cancel: watch::Receiver<bool>,
    tried: &mut Vec<String>,
) -> Result<(String, String), String> {
    let (item, quality) = {
        let st = app.state::<AppState>();
        let q = st.queue.lock().unwrap();
        let e = q.jobs.iter().find(|e| e.view.id == id).ok_or("job vanished")?;
        (e.item.clone(), e.view.quality.clone())
    };
    let settings = app.state::<AppState>().settings.lock().unwrap().clone();
    update(app, id, |v| {
        v.state = if item.url.is_some() { "downloading" } else { "resolving" }.into();
        v.progress = 0.0;
    });

    if settings.skip_existing && !item.title.is_empty() {
        let mut found = find_output(&out_stem(&settings, &item));
        if found.is_none() {
            found = library::find_existing(app, &settings.output_dir, &item).await;
        }
        if let Some(p) = found {
            return Ok((p.to_string_lossy().into_owned(), "Already in library, skipped".into()));
        }
    }

    let url = match item.url.clone().filter(|u| !u.is_empty()) {
        Some(u) => u,
        None => {
            let u = tokio::select! {
                r = ytdlp::resolve(app, &item, tried) => r?,
                _ = cancelled(&mut cancel) => return Err("cancelled".into()),
            };
            update(app, id, |v| v.state = "downloading".into());
            tried.push(u.clone()); // a retry moves on to the next best source
            u
        }
    };

    let exe = tools::find(app, "yt-dlp").ok_or("yt-dlp not found. Install it in Settings > Tools.")?;
    let stem = out_stem(&settings, &item);
    if let Some(d) = stem.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("cannot create {}: {e}", d.display()))?;
    }
    let template = format!(
        "{}.%(ext)s",
        stem.to_string_lossy().replace('%', "%%").replace(TITLE_TOKEN, "%(title)s")
    );

    let mut args: Vec<String> = ytdlp::base_args(app, true);
    args.extend(tools::ffmpeg_args(app));
    args.extend(quality_args(&quality).into_iter().map(String::from));
    args.extend(
        [
            "--no-playlist",
            "--newline",
            "--progress",
            "--retries",
            "5",
            "--concurrent-fragments",
            "4",
            "--progress-template",
            "download:PROG|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s",
            "--print",
            "after_move:FILE:%(filepath)s",
            "-o",
        ]
        .map(String::from),
    );
    args.push(template);
    if item.title.is_empty() {
        args.extend(["--print", "before_dl:TITLE:%(title)s"].map(String::from));
    }
    if !item.catalog {
        args.extend(["--embed-metadata", "--parse-metadata", "title:(?P<artist>.+?) - (?P<title>.+)"].map(String::from));
        if quality != "wav" {
            args.extend(["--embed-thumbnail", "--convert-thumbnails", "jpg"].map(String::from));
        }
    }
    args.push("--".into());
    args.push(url);

    let mut child = tools::command(&exe)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start yt-dlp: {e}"))?;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    pump_lines(child.stdout.take().ok_or("no stdout")?, tx.clone());
    pump_lines(child.stderr.take().ok_or("no stderr")?, tx);

    let mut final_path: Option<PathBuf> = None;
    let mut last_err = String::new();
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    loop {
        tokio::select! {
            line = rx.recv() => {
                let Some(line) = line else { break };
                if let Some(rest) = line.strip_prefix("PROG|") {
                    let f: Vec<Option<f64>> = rest.split('|').map(|x| x.trim().parse().ok()).collect();
                    let dl = f.first().copied().flatten();
                    let total = f.get(1).copied().flatten().or(f.get(2).copied().flatten());
                    if let (Some(d), Some(t)) = (dl, total) {
                        if t > 0.0 && last_emit.elapsed() > Duration::from_millis(250) {
                            last_emit = Instant::now();
                            let speed = f.get(3).copied().flatten().map(fmt_speed).unwrap_or_default();
                            let eta = f.get(4).copied().flatten().map(fmt_eta).unwrap_or_default();
                            update(app, id, |v| {
                                v.progress = (d / t).clamp(0.0, 1.0) * 0.98;
                                v.speed = speed;
                                v.eta = eta;
                            });
                        }
                    }
                } else if let Some(t) = line.strip_prefix("TITLE:") {
                    let t = t.trim().to_string();
                    update(app, id, |v| v.title = t);
                } else if let Some(p) = line.strip_prefix("FILE:") {
                    final_path = Some(PathBuf::from(p.trim()));
                } else if line.starts_with("ERROR") {
                    last_err = ytdlp::last_error(&line);
                } else if line.starts_with("[ExtractAudio]") || line.starts_with("[Fixup") || line.starts_with("[Metadata]") {
                    update(app, id, |v| { v.state = "converting".into(); v.progress = 0.98; v.speed.clear(); v.eta.clear(); });
                }
            }
            _ = cancelled(&mut cancel) => {
                let _ = child.kill().await;
                return Err("cancelled".into());
            }
        }
    }
    let status = child.wait().await.map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(if last_err.is_empty() { format!("yt-dlp exited with {status}") } else { last_err });
    }
    let path = final_path
        .filter(|p| p.exists())
        .or_else(|| find_output(&stem))
        .ok_or("download finished but the output file was not found")?;

    library::remember(app, &path);
    let mut note = String::new();
    if item.catalog {
        update(app, id, |v| { v.state = "tagging".into(); v.progress = 0.99; });
        let mut item = item.clone();
        if item.artwork.is_empty() {
            // Imported tracks carry little metadata: borrow cover and tags from a catalog match.
            if let Some(m) = catalog::find_track(app, &item.artist, &item.title, item.duration_ms).await {
                item.artwork = m.artwork;
                if item.album.is_empty() {
                    item.album = m.album;
                }
                if item.year.is_empty() {
                    item.year = m.year;
                }
                if item.genre.is_empty() {
                    item.genre = m.genre;
                }
            }
        }
        let cover = tagger::cover(app, &item.artwork).await;
        let p = path.clone();
        let it = item.clone();
        let r = tokio::task::spawn_blocking(move || tagger::write_tags(&p, &it, cover.as_deref()))
            .await
            .map_err(|e| e.to_string())?;
        if let Err(e) = r {
            note = format!("saved, but tagging failed: {e}");
        }
    }
    Ok((path.to_string_lossy().into_owned(), note))
}

// ---------------- commands ----------------

#[tauri::command]
pub fn enqueue(app: AppHandle, items: Vec<Item>, quality: String) -> usize {
    add_items(&app, items, &quality)
}

#[tauri::command]
pub async fn enqueue_releases(app: AppHandle, ids: Vec<u64>, quality: String, provider: Option<String>) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let total = ids.len();
        let (mut tracks, mut failed) = (0usize, 0usize);
        for (i, id) in ids.into_iter().enumerate() {
            let _ = app.emit("notice", format!("Reading release {}/{}…", i + 1, total));
            match catalog::fetch_release_from(&app, provider.as_deref(), id).await {
                Ok((_, t)) => tracks += add_items(&app, t.into_iter().map(Item::from).collect(), &quality),
                Err(_) => failed += 1,
            }
            tokio::time::sleep(Duration::from_millis(350)).await;
        }
        let mut msg = format!("Queued {tracks} tracks from {} releases", total - failed);
        if failed > 0 {
            msg.push_str(&format!(" ({failed} could not be read)"));
        }
        let _ = app.emit("notice", msg);
    });
    Ok(())
}

#[tauri::command]
pub fn list_jobs(app: AppHandle) -> Vec<JobView> {
    app.state::<AppState>().queue.lock().unwrap().jobs.iter().map(|e| e.view.clone()).collect()
}

#[tauri::command]
pub fn cancel_job(app: AppHandle, id: u64) {
    let st = app.state::<AppState>();
    let mut emit = None;
    {
        let mut q = st.queue.lock().unwrap();
        if let Some(e) = q.jobs.iter_mut().find(|e| e.view.id == id) {
            if let Some(tx) = &e.cancel {
                let _ = tx.send(true);
            } else if e.view.state == "queued" {
                e.view.state = "cancelled".into();
                emit = Some(e.view.clone());
            }
        }
    }
    if let Some(v) = emit {
        let _ = app.emit("job-update", &v);
        persist(&app);
    }
}

#[tauri::command]
pub fn cancel_all(app: AppHandle) {
    let ids: Vec<u64> = {
        let st = app.state::<AppState>();
        let q = st.queue.lock().unwrap();
        q.jobs
            .iter()
            .filter(|e| matches!(e.view.state.as_str(), "queued" | "resolving" | "downloading" | "converting"))
            .map(|e| e.view.id)
            .collect()
    };
    for id in ids {
        cancel_job(app.clone(), id);
    }
}

#[tauri::command]
pub fn retry_job(app: AppHandle, id: u64) {
    update(&app, id, |v| {
        if matches!(v.state.as_str(), "error" | "cancelled") {
            v.state = "queued".into();
            v.error.clear();
        }
    });
    persist(&app);
    pump(&app);
}

#[tauri::command]
pub fn clear_finished(app: AppHandle) {
    app.state::<AppState>()
        .queue
        .lock()
        .unwrap()
        .jobs
        .retain(|e| !matches!(e.view.state.as_str(), "done" | "error" | "cancelled"));
    persist(&app);
}

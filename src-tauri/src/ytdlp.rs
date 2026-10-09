//! Thin wrapper around yt-dlp: probing links/playlists, web search, preview
//! stream URLs and matching catalog tracks to a downloadable source.
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::{queue::Item, tools, AppState};

#[derive(Serialize, Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub url: String,
    pub duration: Option<f64>,
    pub uploader: String,
    pub thumbnail: String,
    /// Flat listings of some sites carry no titles; `title` is then guessed from the URL.
    pub untitled: bool,
}

/// Readable title from a URL's last path segment ("city-ports" -> "city ports").
pub fn slug_title(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or("");
    let seg = path.trim_end_matches('/').rsplit('/').next().unwrap_or("");
    let t = seg.replace(['-', '_'], " ");
    if t.trim().is_empty() { "Untitled".into() } else { t }
}

/// Split a free-form arg string, honouring double quotes.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn base_args(app: &AppHandle, with_extra: bool) -> Vec<String> {
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    let mut a: Vec<String> = vec!["--no-warnings".into()];
    if !s.cookies_browser.is_empty() {
        a.push("--cookies-from-browser".into());
        a.push(s.cookies_browser.clone());
    }
    if with_extra {
        a.extend(split_args(&s.extra_args));
    }
    a
}

pub fn last_error(stderr: &str) -> String {
    let line = stderr
        .lines()
        .rev()
        .find(|l| l.starts_with("ERROR"))
        .or_else(|| stderr.lines().rev().find(|l| !l.trim().is_empty()))
        .unwrap_or("yt-dlp failed");
    let line = line.trim_start_matches("ERROR:").trim();
    line.chars().take(300).collect()
}

async fn run(app: &AppHandle, args: Vec<String>) -> Result<String, String> {
    let exe = tools::find(app, "yt-dlp")
        .ok_or("yt-dlp not found. Install it in Settings > Tools.")?;
    let out = tools::command(&exe)
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(last_error(&String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn parse_entry(v: &Value) -> Option<Entry> {
    let id = v["id"].as_str().unwrap_or("").to_string();
    let title = v["title"].as_str().unwrap_or("").to_string();
    if title.starts_with("[Private") || title.starts_with("[Deleted") {
        return None;
    }
    let mut url = v["webpage_url"]
        .as_str()
        .or_else(|| v["url"].as_str())
        .unwrap_or("")
        .to_string();
    if !url.starts_with("http") {
        let ie = v["ie_key"].as_str().unwrap_or("").to_lowercase();
        if ie.starts_with("youtube") && !id.is_empty() {
            url = format!("https://www.youtube.com/watch?v={id}");
        } else {
            return None;
        }
    }
    let uploader = ["artist", "uploader", "channel", "creator"]
        .iter()
        .find_map(|k| v[*k].as_str().filter(|s| !s.is_empty()))
        .unwrap_or("")
        .to_string();
    let thumbnail = v["thumbnail"]
        .as_str()
        .map(String::from)
        .or_else(|| {
            v["thumbnails"]
                .as_array()
                .and_then(|a| a.last())
                .and_then(|t| t["url"].as_str().map(String::from))
        })
        .unwrap_or_default();
    let untitled = title.is_empty();
    Some(Entry {
        id,
        title: if untitled { slug_title(&url) } else { title },
        untitled,
        url,
        duration: v["duration"].as_f64(),
        uploader,
        thumbnail,
    })
}

fn flatten(v: &Value, out: &mut Vec<Entry>, depth: u8) {
    if let Some(list) = v["entries"].as_array() {
        for e in list {
            if e["entries"].is_array() && depth < 3 {
                flatten(e, out, depth + 1);
            } else if let Some(x) = parse_entry(e) {
                out.push(x);
            }
        }
    }
}

#[tauri::command]
pub async fn probe_url(app: AppHandle, url: String) -> Result<Value, String> {
    let mut args = base_args(&app, false);
    args.extend(["--flat-playlist".into(), "-J".into(), url.clone()]);
    let txt = run(&app, args).await?;
    let v: Value = serde_json::from_str(&txt).map_err(|e| format!("bad yt-dlp output: {e}"))?;
    let mut entries = vec![];
    let is_playlist = v["entries"].is_array();
    if is_playlist {
        flatten(&v, &mut entries, 0);
    } else if let Some(mut e) = parse_entry(&v) {
        if e.url.is_empty() {
            e.url = url.clone();
        }
        entries.push(e);
    }
    if entries.is_empty() {
        return Err("Nothing downloadable found at this link".into());
    }
    let title = v["title"].as_str().unwrap_or("").to_string();
    let uploader = ["uploader", "channel", "artist"]
        .iter()
        .find_map(|k| v[*k].as_str().filter(|s| !s.is_empty()))
        .unwrap_or("")
        .to_string();
    Ok(json!({
        "url": url,
        "title": title,
        "uploader": uploader,
        "is_playlist": is_playlist,
        "extractor": v["extractor_key"].as_str().unwrap_or(""),
        "entries": entries,
    }))
}

#[tauri::command]
pub async fn search_web(app: AppHandle, provider: String, term: String) -> Result<Vec<Entry>, String> {
    let query = match provider.as_str() {
        "soundcloud" => format!("scsearch30:{term}"),
        "ytmusic" => {
            let q = reqwest::Url::parse_with_params("https://music.youtube.com/search", &[("q", term.as_str())])
                .map_err(|e| e.to_string())?;
            format!("{q}#songs")
        }
        _ => format!("ytsearch30:{term}"),
    };
    let mut args = base_args(&app, false);
    args.extend(["--flat-playlist".into(), "--playlist-end".into(), "30".into(), "-J".into(), query]);
    let txt = run(&app, args).await?;
    let v: Value = serde_json::from_str(&txt).map_err(|e| e.to_string())?;
    let mut out = vec![];
    flatten(&v, &mut out, 0);
    Ok(out)
}

#[tauri::command]
pub async fn stream_url(app: AppHandle, url: String) -> Result<String, String> {
    let mut args = base_args(&app, false);
    args.extend([
        "-g".into(),
        "--no-playlist".into(),
        "-f".into(),
        "bestaudio[protocol=https]/bestaudio[protocol=http]/bestaudio".into(),
        url,
    ]);
    let txt = run(&app, args).await?;
    txt.lines()
        .find(|l| l.starts_with("http"))
        .map(String::from)
        .ok_or_else(|| "No playable stream".into())
}

// ---------- matching a catalog track to a downloadable source ----------

fn norm(s: &str) -> String {
    let mut depth = 0i32;
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            c if depth == 0 && c.is_alphanumeric() => out.push(c),
            c if depth == 0 && c.is_whitespace() => out.push(' '),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

const BAD_WORDS: &[&str] = &[
    "live", "remix", "cover", "karaoke", "instrumental", "slowed", "sped up", "speed up",
    "reverb", "nightcore", "8d", "tribute", "mashup", "reaction", "acoustic", "extended",
    "full album", "bass boosted", "1 hour", "loop",
];

fn score(e: &Entry, it: &Item) -> i32 {
    let mut sc = 0;
    if it.duration_ms > 0 {
        if let Some(d) = e.duration {
            let diff = (d - it.duration_ms as f64 / 1000.0).abs();
            sc += match diff {
                x if x <= 2.0 => 50,
                x if x <= 5.0 => 40,
                x if x <= 10.0 => 15,
                x if x <= 20.0 => -20,
                _ => -50,
            };
        }
    }
    let title = norm(&e.title);
    let up = norm(&e.uploader);
    let want_title = norm(&it.title);
    let first_artist = norm(it.artist.split(&[',', '&'][..]).next().unwrap_or(""));
    if !want_title.is_empty() && title.contains(&want_title) {
        sc += 25;
    }
    if !first_artist.is_empty() && (title.contains(&first_artist) || up.contains(&first_artist)) {
        sc += 15;
    }
    if e.uploader.ends_with("- Topic") {
        sc += 20;
    }
    let lt = e.title.to_lowercase();
    if lt.contains("official audio") {
        sc += 8;
    }
    let wanted = format!("{} {}", it.title, it.album).to_lowercase();
    for w in BAD_WORDS {
        if lt.split(|c: char| !c.is_alphanumeric()).collect::<Vec<_>>().join(" ").contains(w)
            && !wanted.contains(w)
        {
            sc -= 40;
            break;
        }
    }
    sc
}

/// Find the best downloadable source URL for a catalog track.
pub async fn resolve(app: &AppHandle, it: &Item, tried: &[String]) -> Result<String, String> {
    let query = format!("{} {}", it.artist, it.title);
    let mut last_err = String::new();
    for prefix in ["ytsearch10", "scsearch8"] {
        let mut args = base_args(app, false);
        args.extend([
            "--flat-playlist".into(),
            "-J".into(),
            format!("{prefix}:{query}"),
        ]);
        let txt = match run(app, args).await {
            Ok(t) => t,
            Err(e) => {
                last_err = e;
                continue;
            }
        };
        let Ok(v) = serde_json::from_str::<Value>(&txt) else { continue };
        let mut entries = vec![];
        flatten(&v, &mut entries, 0);
        let mut best: Option<(i32, &Entry)> = None;
        for e in entries.iter().filter(|e| !tried.contains(&e.url)) {
            let sc = score(e, it);
            if best.map_or(true, |(b, _)| sc > b) {
                best = Some((sc, e));
            }
        }
        if let Some((sc, e)) = best {
            if sc >= 40 {
                return Ok(e.url.clone());
            }
        }
    }
    Err(if last_err.is_empty() {
        "No matching source found".into()
    } else {
        last_err
    })
}

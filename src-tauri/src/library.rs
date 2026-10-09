//! Index of audio files already on disk, so downloads can skip what exists.
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

use crate::{queue::Item, AppState};

const EXTS: [&str; 9] = ["mp3", "flac", "m4a", "opus", "ogg", "wav", "aac", "webm", "mka"];
const MAX_AGE: Duration = Duration::from_secs(120);
const PREFIX: &str = r"^\s*(\d+\s*[-.]\s*)?\d+\s*[-._)]\s*";

#[derive(Default, Clone)]
pub struct Library {
    /// Normalised file name (track number stripped) -> (normalised full path, path).
    files: HashMap<String, Vec<(String, PathBuf)>>,
    count: usize,
}

/// (scanned at, folder scanned, index)
pub type Cache = Option<(Instant, String, Library)>;

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

fn is_audio(p: &Path) -> bool {
    p.extension().map_or(false, |x| EXTS.contains(&x.to_string_lossy().to_lowercase().as_str()))
}

impl Library {
    fn scan(dir: &Path) -> Library {
        let prefix = Regex::new(PREFIX).unwrap();
        let mut lib = Library::default();
        let mut stack = vec![(dir.to_path_buf(), 0u8)];
        while let Some((d, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let Ok(ft) = e.file_type() else { continue };
                if ft.is_dir() {
                    if depth < 8 {
                        stack.push((p, depth + 1));
                    }
                } else if ft.is_file() && is_audio(&p) {
                    lib.add(&p, &prefix);
                }
            }
        }
        lib
    }

    fn add(&mut self, p: &Path, prefix: &Regex) {
        let Some(stem) = p.file_stem().map(|s| s.to_string_lossy().into_owned()) else { return };
        let stripped = prefix.replace(&stem, "").into_owned();
        let full = norm(&p.to_string_lossy());
        self.count += 1;
        self.files.entry(norm(&stripped)).or_default().push((full, p.to_path_buf()));
    }

    fn find(&self, it: &Item) -> Option<PathBuf> {
        let (t, a) = (norm(&it.title), norm(&it.artist));
        if t.is_empty() || a.is_empty() {
            return None;
        }
        if let Some(v) = self.files.get(&format!("{a}{t}")) {
            return v.first().map(|x| x.1.clone());
        }
        self.files.get(&t)?.iter().find(|(full, _)| full.contains(&a)).map(|x| x.1.clone())
    }
}

/// Looks the item up in the cached index of the download folder, rescanning when stale.
pub async fn find_existing(app: &AppHandle, dir: &str, it: &Item) -> Option<PathBuf> {
    {
        let st = app.state::<AppState>();
        let c = st.library.lock().unwrap();
        if let Some((at, d, lib)) = c.as_ref() {
            if d == dir && at.elapsed() < MAX_AGE {
                return lib.find(it);
            }
        }
    }
    refresh(app, dir).await.find(it)
}

async fn refresh(app: &AppHandle, dir: &str) -> Library {
    let d = PathBuf::from(dir);
    let lib = tokio::task::spawn_blocking(move || Library::scan(&d)).await.unwrap_or_default();
    *app.state::<AppState>().library.lock().unwrap() = Some((Instant::now(), dir.to_string(), lib.clone()));
    lib
}

/// Records a freshly downloaded file so later jobs in the same session see it.
pub fn remember(app: &AppHandle, p: &Path) {
    let st = app.state::<AppState>();
    let mut c = st.library.lock().unwrap();
    if let Some((_, _, lib)) = c.as_mut() {
        lib.add(p, &Regex::new(PREFIX).unwrap());
    }
}

#[tauri::command]
pub async fn scan_library(app: AppHandle) -> usize {
    let dir = app.state::<AppState>().settings.lock().unwrap().output_dir.clone();
    refresh(&app, &dir).await.count
}

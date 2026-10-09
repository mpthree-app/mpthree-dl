use futures_util::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

use crate::AppState;

pub fn tools_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("bin")
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// An own copy in the app's bin dir wins over anything on PATH.
pub fn find(app: &AppHandle, name: &str) -> Option<PathBuf> {
    let own = tools_dir(app).join(exe(name));
    if own.exists() {
        return Some(own);
    }
    which::which(name).ok()
}

pub fn command(path: &Path) -> tokio::process::Command {
    let mut c = tokio::process::Command::new(path);
    #[cfg(windows)]
    c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    c.env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::null())
        .kill_on_drop(true);
    c
}

/// Extra yt-dlp args so it finds an ffmpeg we installed ourselves.
pub fn ffmpeg_args(app: &AppHandle) -> Vec<String> {
    let dir = tools_dir(app);
    if dir.join(exe("ffmpeg")).exists() {
        vec!["--ffmpeg-location".into(), dir.to_string_lossy().into_owned()]
    } else {
        vec![]
    }
}

#[derive(Serialize)]
pub struct ToolInfo {
    path: String,
    version: String,
}

#[derive(Serialize)]
pub struct ToolsStatus {
    ytdlp: Option<ToolInfo>,
    ffmpeg: Option<ToolInfo>,
    platform: &'static str,
}

async fn version_of(path: &Path, arg: &str) -> String {
    match command(path).arg(arg).output().await {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string(),
        Err(_) => String::new(),
    }
}

#[tauri::command]
pub async fn tools_status(app: AppHandle) -> ToolsStatus {
    let ytdlp = match find(&app, "yt-dlp") {
        Some(p) => Some(ToolInfo {
            version: version_of(&p, "--version").await,
            path: p.to_string_lossy().into_owned(),
        }),
        None => None,
    };
    let ffmpeg = match find(&app, "ffmpeg") {
        Some(p) => {
            let v = version_of(&p, "-version").await;
            Some(ToolInfo {
                version: v.split_whitespace().nth(2).unwrap_or("").to_string(),
                path: p.to_string_lossy().into_owned(),
            })
        }
        None => None,
    };
    ToolsStatus {
        ytdlp,
        ffmpeg,
        platform: std::env::consts::OS,
    }
}

async fn download(app: &AppHandle, tool: &str, url: &str, dest: &Path) -> Result<(), String> {
    let http = app.state::<AppState>().http.clone();
    let resp = http.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("download failed: HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    if let Some(d) = dest.parent() {
        tokio::fs::create_dir_all(d).await.map_err(|e| e.to_string())?;
    }
    let mut file = tokio::fs::File::create(dest).await.map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut got = 0u64;
    let mut last = -1i64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        got += chunk.len() as u64;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        if total > 0 {
            let pct = (got * 100 / total) as i64;
            if pct != last {
                last = pct;
                let _ = app.emit("tool-progress", serde_json::json!({ "tool": tool, "pct": pct }));
            }
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn install_ytdlp(app: AppHandle) -> Result<(), String> {
    let (url, name) = match std::env::consts::OS {
        "windows" => ("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe", "yt-dlp.exe"),
        "macos" => ("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos", "yt-dlp"),
        _ => ("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp", "yt-dlp"),
    };
    let dest = tools_dir(&app).join(name);
    let tmp = dest.with_extension("part");
    download(&app, "ytdlp", url, &tmp).await?;
    tokio::fs::rename(&tmp, &dest).await.map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

#[tauri::command]
pub async fn update_ytdlp(app: AppHandle) -> Result<String, String> {
    let own = tools_dir(&app).join(exe("yt-dlp"));
    if own.exists() {
        let out = command(&own).arg("-U").output().await.map_err(|e| e.to_string())?;
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    // A package-manager copy can't self-update; fetch our own copy that takes priority.
    install_ytdlp(app).await?;
    Ok("Installed latest yt-dlp".into())
}

#[tauri::command]
pub async fn install_ffmpeg(app: AppHandle) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Install ffmpeg with your package manager (brew / apt / pacman).".into());
    }
    let dir = tools_dir(&app);
    let zip_path = dir.join("ffmpeg.zip");
    download(
        &app,
        "ffmpeg",
        "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
        &zip_path,
    )
    .await?;
    let d = dir.clone();
    let zp = zip_path.clone();
    let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut ar = zip::ZipArchive::new(std::fs::File::open(&zp).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        for i in 0..ar.len() {
            let mut f = ar.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().to_string();
            let leaf = name.rsplit('/').next().unwrap_or("");
            if name.contains("/bin/") && (leaf == "ffmpeg.exe" || leaf == "ffprobe.exe") {
                let mut out = std::fs::File::create(d.join(leaf)).map_err(|e| e.to_string())?;
                std::io::copy(&mut f, &mut out).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?;
    let _ = tokio::fs::remove_file(&zip_path).await;
    res
}

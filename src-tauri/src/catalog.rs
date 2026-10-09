//! Music catalog lookups (iTunes Search API): artists, releases, songs, with
//! cover art and 30s previews. The audio itself is fetched through yt-dlp.
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use crate::{queue::Item, AppState};

#[derive(Serialize, Clone)]
pub struct Artist {
    id: u64,
    name: String,
    genre: String,
}

#[derive(Serialize, Clone)]
pub struct Release {
    pub id: u64,
    pub name: String,
    pub artist: String,
    pub artist_id: u64,
    pub artwork: String,
    pub track_count: u32,
    pub year: String,
    pub kind: String,
    pub explicit: bool,
    pub genre: String,
}

#[derive(Serialize, Clone)]
pub struct Track {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_id: u64,
    pub album_artist: String,
    pub artwork: String,
    pub preview_url: String,
    pub track_no: u32,
    pub track_total: u32,
    pub disc_no: u32,
    pub disc_total: u32,
    pub duration_ms: u64,
    pub year: String,
    pub genre: String,
    pub explicit: bool,
}

impl From<Track> for Item {
    fn from(t: Track) -> Item {
        Item {
            url: None,
            catalog: true,
            title: t.title,
            artist: t.artist,
            album: t.album,
            album_artist: t.album_artist,
            track_no: t.track_no,
            track_total: t.track_total,
            disc_no: t.disc_no,
            disc_total: t.disc_total,
            year: t.year,
            genre: t.genre,
            artwork: t.artwork,
            duration_ms: t.duration_ms,
            subdir: None,
            index: None,
        }
    }
}

/// Resize an iTunes artwork URL (they all contain `100x100bb`).
pub fn art(url: &str, size: u32) -> String {
    url.replace("100x100bb", &format!("{size}x{size}bb"))
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}
fn n(v: &Value, k: &str) -> u32 {
    v[k].as_u64().unwrap_or(0) as u32
}
fn year(v: &Value) -> String {
    s(v, "releaseDate").chars().take(4).collect()
}

fn split_kind(raw: &str) -> (String, &'static str) {
    if let Some(x) = raw.strip_suffix(" - Single") {
        (x.to_string(), "single")
    } else if let Some(x) = raw.strip_suffix(" - EP") {
        (x.to_string(), "ep")
    } else {
        (raw.to_string(), "album")
    }
}

fn parse_artist(v: &Value) -> Option<Artist> {
    Some(Artist {
        id: v["artistId"].as_u64()?,
        name: s(v, "artistName"),
        genre: s(v, "primaryGenreName"),
    })
}

fn parse_release(v: &Value) -> Option<Release> {
    let id = v["collectionId"].as_u64()?;
    let (name, kind) = split_kind(&s(v, "collectionName"));
    Some(Release {
        id,
        name,
        artist: s(v, "artistName"),
        artist_id: v["artistId"].as_u64().unwrap_or(0),
        artwork: s(v, "artworkUrl100"),
        track_count: n(v, "trackCount"),
        year: year(v),
        kind: kind.into(),
        explicit: s(v, "collectionExplicitness") == "explicit",
        genre: s(v, "primaryGenreName"),
    })
}

fn parse_track(v: &Value, fallback_album_artist: &str) -> Option<Track> {
    if v["kind"].as_str() != Some("song") {
        return None;
    }
    let artist = s(v, "artistName");
    let mut aa = s(v, "collectionArtistName");
    if aa.is_empty() {
        aa = if fallback_album_artist.is_empty() {
            artist.clone()
        } else {
            fallback_album_artist.into()
        };
    }
    Some(Track {
        id: v["trackId"].as_u64()?,
        title: s(v, "trackName"),
        artist,
        album: split_kind(&s(v, "collectionName")).0,
        album_id: v["collectionId"].as_u64().unwrap_or(0),
        album_artist: aa,
        artwork: s(v, "artworkUrl100"),
        preview_url: s(v, "previewUrl"),
        track_no: n(v, "trackNumber"),
        track_total: n(v, "trackCount"),
        disc_no: n(v, "discNumber").max(1),
        disc_total: n(v, "discCount").max(1),
        duration_ms: v["trackTimeMillis"].as_u64().unwrap_or(0),
        year: year(v),
        genre: s(v, "primaryGenreName"),
        explicit: s(v, "trackExplicitness") == "explicit",
    })
}

fn country(app: &AppHandle) -> String {
    app.state::<AppState>().settings.lock().unwrap().country.clone()
}

pub(crate) async fn get_json(app: &AppHandle, url: &str, params: &[(&str, String)]) -> Result<Value, String> {
    let http = app.state::<AppState>().http.clone();
    let url = reqwest::Url::parse_with_params(url, params).map_err(|e| e.to_string())?;
    let mut last = String::new();
    for attempt in 0..4u64 {
        match http.get(url.clone()).send().await {
            Ok(r) if r.status().is_success() => return r.json::<Value>().await.map_err(|e| e.to_string()),
            Ok(r) => last = format!("HTTP {}", r.status()),
            Err(e) => last = e.to_string(),
        }
        tokio::time::sleep(Duration::from_millis(800 * (attempt + 1))).await;
    }
    Err(format!("Catalog request failed ({last})"))
}

fn results(v: &Value) -> Vec<Value> {
    v["results"].as_array().cloned().unwrap_or_default()
}

#[tauri::command]
pub async fn search_catalog(app: AppHandle, kind: String, term: String, provider: Option<String>) -> Result<Value, String> {
    if provider.as_deref() == Some("deezer") {
        return deezer::search(&app, &kind, &term).await;
    }
    let entity = match kind.as_str() {
        "artists" => "musicArtist",
        "releases" => "album",
        _ => "song",
    };
    let v = get_json(
        &app,
        "https://itunes.apple.com/search",
        &[
            ("term", term),
            ("media", "music".into()),
            ("entity", entity.into()),
            ("limit", "50".into()),
            ("country", country(&app)),
        ],
    )
    .await?;
    let r = results(&v);
    Ok(match kind.as_str() {
        "artists" => json!(r.iter().filter_map(parse_artist).collect::<Vec<_>>()),
        "releases" => json!(r.iter().filter_map(parse_release).collect::<Vec<_>>()),
        _ => json!(r.iter().filter_map(|x| parse_track(x, "")).collect::<Vec<_>>()),
    })
}

#[tauri::command]
pub async fn artist_releases(app: AppHandle, artist_id: u64, provider: Option<String>) -> Result<Value, String> {
    if provider.as_deref() == Some("deezer") {
        return deezer::artist_releases(&app, artist_id).await;
    }
    let v = get_json(
        &app,
        "https://itunes.apple.com/lookup",
        &[
            ("id", artist_id.to_string()),
            ("entity", "album".into()),
            ("limit", "200".into()),
            ("country", country(&app)),
        ],
    )
    .await?;
    let r = results(&v);
    let artist = r.iter().find(|x| x["wrapperType"] == "artist").and_then(parse_artist);
    let mut rel: Vec<Release> = r
        .iter()
        .filter(|x| x["wrapperType"] == "collection")
        .filter_map(parse_release)
        .collect();
    rel.sort_by(|a, b| b.year.cmp(&a.year));
    Ok(json!({ "artist": artist, "releases": rel }))
}

pub async fn fetch_release_from(app: &AppHandle, provider: Option<&str>, id: u64) -> Result<(Release, Vec<Track>), String> {
    if provider == Some("deezer") {
        return deezer::fetch_release(app, id).await;
    }
    fetch_release(app, id).await
}

pub async fn fetch_release(app: &AppHandle, id: u64) -> Result<(Release, Vec<Track>), String> {
    let v = get_json(
        app,
        "https://itunes.apple.com/lookup",
        &[
            ("id", id.to_string()),
            ("entity", "song".into()),
            ("limit", "200".into()),
            ("country", country(app)),
        ],
    )
    .await?;
    let r = results(&v);
    let rel = r
        .iter()
        .find(|x| x["wrapperType"] == "collection")
        .and_then(parse_release)
        .ok_or("Release not found in this store country")?;
    let mut tracks: Vec<Track> = r.iter().filter_map(|x| parse_track(x, &rel.artist)).collect();
    tracks.sort_by_key(|t| (t.disc_no, t.track_no));
    Ok((rel, tracks))
}

#[tauri::command]
pub async fn release_tracks(app: AppHandle, release_id: u64, provider: Option<String>) -> Result<Value, String> {
    let (release, tracks) = fetch_release_from(&app, provider.as_deref(), release_id).await?;
    Ok(json!({ "release": release, "tracks": tracks }))
}

/// Full metadata for known iTunes track ids (order not guaranteed). `cc` is the storefront.
pub async fn lookup_tracks(app: &AppHandle, ids: &[u64], cc: &str) -> Result<Vec<Track>, String> {
    let list: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    let v = get_json(
        app,
        "https://itunes.apple.com/lookup",
        &[("id", list.join(",")), ("entity", "song".into()), ("country", cc.to_string())],
    )
    .await?;
    Ok(results(&v).iter().filter_map(|x| parse_track(x, "")).collect())
}

fn norm_title(s: &str) -> String {
    let mut depth = 0;
    s.to_lowercase()
        .chars()
        .filter(|c| match c {
            '(' | '[' => { depth += 1; false }
            ')' | ']' => { depth = (depth - 1).max(0); false }
            c => depth == 0 && c.is_alphanumeric(),
        })
        .collect()
}

/// Best catalog match for an artist/title pair, used to fill in cover art and tags.
pub async fn find_track(app: &AppHandle, artist: &str, title: &str, duration_ms: u64) -> Option<Track> {
    let v = get_json(
        app,
        "https://itunes.apple.com/search",
        &[
            ("term", format!("{artist} {title}")),
            ("media", "music".into()),
            ("entity", "song".into()),
            ("limit", "8".into()),
            ("country", country(app)),
        ],
    )
    .await
    .ok()?;
    let want = norm_title(title);
    results(&v).iter().filter_map(|x| parse_track(x, "")).find(|t| {
        norm_title(&t.title) == want
            && (duration_ms == 0 || t.duration_ms == 0 || t.duration_ms.abs_diff(duration_ms) <= 6000)
    })
}

/// Deezer's public API (no login): same shapes as the iTunes catalog above.
mod deezer {
    use super::*;

    const API: &str = "https://api.deezer.com";

    async fn get(app: &AppHandle, path: &str, params: &[(&str, String)]) -> Result<Value, String> {
        let v = get_json(app, &format!("{API}/{path}"), params).await?;
        if v["error"].is_object() {
            return Err(format!("Deezer: {}", s(&v["error"], "message")));
        }
        Ok(v)
    }

    fn id(v: &Value) -> u64 {
        v["id"].as_u64().unwrap_or(0)
    }

    fn kind(raw: &str) -> String {
        match raw {
            "ep" => "ep",
            "single" => "single",
            _ => "album",
        }
        .into()
    }

    fn release(v: &Value, artist: &str) -> Release {
        let a = if v["artist"]["name"].is_string() { s(&v["artist"], "name") } else { artist.to_string() };
        Release {
            id: id(v),
            name: s(v, "title"),
            artist: a,
            artist_id: id(&v["artist"]),
            artwork: s(v, "cover_xl"),
            track_count: n(v, "nb_tracks"),
            year: s(v, "release_date").chars().take(4).collect(),
            kind: kind(&s(v, "record_type")),
            explicit: v["explicit_lyrics"].as_bool().unwrap_or(false),
            genre: v["genres"]["data"][0]["name"].as_str().unwrap_or("").to_string(),
        }
    }

    fn track(v: &Value, rel: Option<&Release>, total: u32) -> Track {
        let artist = s(&v["artist"], "name");
        Track {
            id: id(v),
            title: s(v, "title"),
            album: rel.map(|r| r.name.clone()).unwrap_or_else(|| s(&v["album"], "title")),
            album_id: rel.map(|r| r.id).unwrap_or_else(|| id(&v["album"])),
            album_artist: rel.map(|r| r.artist.clone()).unwrap_or_else(|| artist.clone()),
            artist,
            artwork: rel.map(|r| r.artwork.clone()).unwrap_or_else(|| s(&v["album"], "cover_xl")),
            preview_url: s(v, "preview"),
            track_no: n(v, "track_position"),
            track_total: total,
            disc_no: n(v, "disk_number").max(1),
            disc_total: 1,
            duration_ms: v["duration"].as_u64().unwrap_or(0) * 1000,
            year: rel.map(|r| r.year.clone()).unwrap_or_default(),
            genre: rel.map(|r| r.genre.clone()).unwrap_or_default(),
            explicit: v["explicit_lyrics"].as_bool().unwrap_or(false),
        }
    }

    fn data(v: &Value) -> Vec<Value> {
        v["data"].as_array().cloned().unwrap_or_default()
    }

    pub async fn search(app: &AppHandle, kind_: &str, term: &str) -> Result<Value, String> {
        let path = match kind_ {
            "artists" => "search/artist",
            "releases" => "search/album",
            _ => "search/track",
        };
        let v = get(app, path, &[("q", term.to_string()), ("limit", "50".into())]).await?;
        let r = data(&v);
        Ok(match kind_ {
            "artists" => json!(r
                .iter()
                .map(|a| Artist { id: id(a), name: s(a, "name"), genre: String::new() })
                .collect::<Vec<_>>()),
            "releases" => json!(r.iter().map(|a| release(a, "")).collect::<Vec<_>>()),
            _ => json!(r.iter().map(|t| track(t, None, 0)).collect::<Vec<_>>()),
        })
    }

    pub async fn artist_releases(app: &AppHandle, artist_id: u64) -> Result<Value, String> {
        let a = get(app, &format!("artist/{artist_id}"), &[]).await?;
        let name = s(&a, "name");
        let v = get(app, &format!("artist/{artist_id}/albums"), &[("limit", "500".into())]).await?;
        let mut rel: Vec<Release> = data(&v).iter().map(|x| release(x, &name)).collect();
        rel.sort_by(|a, b| b.year.cmp(&a.year));
        Ok(json!({ "artist": Artist { id: artist_id, name, genre: String::new() }, "releases": rel }))
    }

    pub async fn fetch_release(app: &AppHandle, album_id: u64) -> Result<(Release, Vec<Track>), String> {
        let meta = get(app, &format!("album/{album_id}"), &[]).await?;
        let rel = release(&meta, "");
        let v = get(app, &format!("album/{album_id}/tracks"), &[("limit", "500".into())]).await?;
        let items = data(&v);
        let total = items.len() as u32;
        let mut tracks: Vec<Track> = items.iter().map(|t| track(t, Some(&rel), total)).collect();
        let discs = tracks.iter().map(|t| t.disc_no).max().unwrap_or(1);
        tracks.iter_mut().for_each(|t| t.disc_total = discs);
        tracks.sort_by_key(|t| (t.disc_no, t.track_no));
        Ok((rel, tracks))
    }
}

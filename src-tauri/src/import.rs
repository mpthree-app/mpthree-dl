//! Read playlist / album / track metadata from streaming services. Only
//! metadata is read (no audio): the queue later searches YouTube/SoundCloud for each track.
use regex::Regex;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::{catalog, AppState};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

#[derive(Default, Clone)]
struct T {
    title: String,
    artist: String,
    album: String,
    duration_ms: u64,
    artwork: String,
}

struct L {
    source: &'static str,
    title: String,
    owner: String,
    tracks: Vec<T>,
    note: String,
}

async fn get(app: &AppHandle, url: &str, headers: &[(&str, &str)]) -> Result<reqwest::Response, String> {
    let http = app.state::<AppState>().http.clone();
    let mut r = http.get(url);
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("user-agent")) {
        r = r.header("User-Agent", UA);
    }
    for (k, v) in headers {
        r = r.header(*k, *v);
    }
    r.send().await.map_err(|e| format!("network error: {e}"))
}

async fn get_text(app: &AppHandle, url: &str, headers: &[(&str, &str)]) -> Result<String, String> {
    let r = get(app, url, headers).await?;
    if !r.status().is_success() {
        return Err(format!("HTTP {} from {}", r.status(), r.url().host_str().unwrap_or("server")));
    }
    r.text().await.map_err(|e| e.to_string())
}

async fn get_json(app: &AppHandle, url: &str, headers: &[(&str, &str)]) -> Result<Value, String> {
    let r = get(app, url, headers).await?;
    let status = r.status();
    let v: Value = r.json().await.map_err(|e| format!("unexpected response (HTTP {status}): {e}"))?;
    Ok(v)
}

/// Follow short links (spotify.link, deezer.page.link, ...) to the real URL.
async fn follow(app: &AppHandle, url: &str) -> String {
    match get(app, url, &[]).await {
        Ok(r) => r.url().to_string(),
        Err(_) => url.to_string(),
    }
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

// ---------------- Spotify (public embed page + web-player GraphQL) ----------------

static SPOTIFY_HASH: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
const SPOTIFY_HASH_FALLBACK: &str = "8964e8eafb21aa992a7d951d256d83285c04be2105d209262901de70cb97584a";

/// The web player's persisted-query hash for `fetchPlaylist`, read from its JS bundle
/// (it changes between releases) and cached.
async fn spotify_hash(app: &AppHandle, id: &str) -> String {
    if let Some(h) = SPOTIFY_HASH.lock().unwrap().clone() {
        return h;
    }
    let found = async {
        let html = get_text(app, &format!("https://open.spotify.com/playlist/{id}"), &[]).await.ok()?;
        let js = Regex::new(r#"src="(https://open\.spotifycdn\.com/cdn/build/web-player/web-player\.[^"]+\.js)""#)
            .unwrap()
            .captures(&html)?[1]
            .to_string();
        let code = get_text(app, &js, &[]).await.ok()?;
        Some(
            Regex::new(r#""fetchPlaylist","query","([0-9a-f]{64})""#)
                .unwrap()
                .captures(&code)?[1]
                .to_string(),
        )
    }
    .await
    .unwrap_or_else(|| SPOTIFY_HASH_FALLBACK.to_string());
    *SPOTIFY_HASH.lock().unwrap() = Some(found.clone());
    found
}

/// Every track of a playlist, paged 100 at a time, using the anonymous token the embed page hands out.
async fn spotify_playlist_full(app: &AppHandle, id: &str, token: &str) -> Result<Vec<T>, String> {
    let mut hash = spotify_hash(app, id).await;
    let auth = format!("Bearer {token}");
    let mut tracks: Vec<T> = vec![];
    let mut offset = 0usize;
    let mut refreshed = false;
    loop {
        let vars = json!({"uri": format!("spotify:playlist:{id}"), "offset": offset, "limit": 100, "enableWatchFeedEntrypoint": false}).to_string();
        let ext = json!({"persistedQuery": {"version": 1, "sha256Hash": hash}}).to_string();
        let url = reqwest::Url::parse_with_params(
            "https://api-partner.spotify.com/pathfinder/v1/query",
            &[("operationName", "fetchPlaylist"), ("variables", &vars), ("extensions", &ext)],
        )
        .map_err(|e| e.to_string())?;
        let v = get_json(app, url.as_str(), &[("Authorization", &auth), ("app-platform", "WebPlayer")]).await?;
        let pl = &v["data"]["playlistV2"];
        if pl.is_null() {
            // Stale hash: drop the cache and rediscover once.
            if !refreshed {
                refreshed = true;
                *SPOTIFY_HASH.lock().unwrap() = None;
                hash = spotify_hash(app, id).await;
                continue;
            }
            return Err("Spotify query failed".into());
        }
        let items = pl["content"]["items"].as_array().cloned().unwrap_or_default();
        let total = pl["content"]["totalCount"].as_u64().unwrap_or(0) as usize;
        if items.is_empty() {
            break;
        }
        offset += items.len();
        for it in &items {
            let d = &it["itemV2"]["data"];
            if d["__typename"] != "Track" {
                continue;
            }
            let artists: Vec<String> = d["artists"]["items"]
                .as_array()
                .map(|a| a.iter().map(|x| s(&x["profile"]["name"])).collect())
                .unwrap_or_default();
            let best_cover = d["albumOfTrack"]["coverArt"]["sources"]
                .as_array()
                .and_then(|a| a.iter().max_by_key(|x| x["width"].as_u64().unwrap_or(0)))
                .map(|x| s(&x["url"]))
                .unwrap_or_default();
            tracks.push(T {
                title: s(&d["name"]),
                artist: artists.join(", "),
                album: s(&d["albumOfTrack"]["name"]),
                duration_ms: d["trackDuration"]["totalMilliseconds"].as_u64().unwrap_or(0),
                artwork: best_cover,
            });
        }
        if offset >= total || offset >= 10_000 {
            break;
        }
    }
    Ok(tracks)
}

async fn spotify(app: &AppHandle, url: &str) -> Result<L, String> {
    let mut url = url.to_string();
    if url.contains("spotify.link") {
        url = follow(app, &url).await;
    }
    let re = Regex::new(r"spotify\.com/(?:intl-[a-z-]+/)?(playlist|album|track|artist|show|episode)/([A-Za-z0-9]+)").unwrap();
    let c = re.captures(&url).ok_or("Unrecognised Spotify link")?;
    let (kind, id) = (&c[1], &c[2]);
    if kind != "playlist" && kind != "album" && kind != "track" {
        return Err("Only Spotify playlists, albums and tracks are supported. For an artist, use Search > Catalog.".into());
    }
    let html = get_text(app, &format!("https://open.spotify.com/embed/{kind}/{id}"), &[]).await?;
    let start = html
        .find("id=\"__NEXT_DATA__\"")
        .and_then(|i| html[i..].find('>').map(|j| i + j + 1))
        .ok_or("Spotify page had no data (is the playlist public?)")?;
    let end = html[start..].find("</script>").ok_or("bad Spotify page")? + start;
    let v: Value = serde_json::from_str(&html[start..end]).map_err(|e| e.to_string())?;
    let e = &v["props"]["pageProps"]["state"]["data"]["entity"];
    if e.is_null() {
        return Err("Spotify returned no data for this link (private or removed?)".into());
    }
    let title = if e["name"].is_string() { s(&e["name"]) } else { s(&e["title"]) };
    let art = s(&e["coverArt"]["sources"][0]["url"]);
    let clean = |x: &str| x.replace('\u{a0}', " ");
    let album = if kind == "album" { title.clone() } else { String::new() };
    let mut tracks = vec![];
    if let Some(list) = e["trackList"].as_array() {
        for t in list {
            if t["entityType"].as_str().map_or(false, |x| x != "track") {
                continue;
            }
            tracks.push(T {
                title: s(&t["title"]),
                artist: clean(&s(&t["subtitle"])),
                album: album.clone(),
                duration_ms: t["duration"].as_u64().unwrap_or(0),
                artwork: if kind == "album" { art.clone() } else { String::new() },
            });
        }
    } else {
        let artists: Vec<String> = e["artists"]
            .as_array()
            .map(|a| a.iter().map(|x| s(&x["name"])).collect())
            .unwrap_or_default();
        tracks.push(T {
            title: title.clone(),
            artist: if artists.is_empty() { clean(&s(&e["subtitle"])) } else { artists.join(", ") },
            duration_ms: e["duration"].as_u64().unwrap_or(0),
            artwork: art.clone(),
            ..Default::default()
        });
    }
    let mut note = String::new();
    if kind == "playlist" {
        let token = v["props"]["pageProps"]["state"]["settings"]["session"]["accessToken"].as_str().unwrap_or("");
        match spotify_playlist_full(app, id, token).await {
            Ok(full) if full.len() >= tracks.len() && !full.is_empty() => tracks = full,
            _ if tracks.len() >= 100 => note = "Only the first 100 tracks could be read from Spotify this time.".into(),
            _ => {}
        }
    }
    Ok(L {
        source: "Spotify",
        title,
        owner: clean(&s(&e["subtitle"])),
        tracks,
        note,
    })
}

// ---------------- Deezer (public API) ----------------

async fn deezer(app: &AppHandle, url: &str) -> Result<L, String> {
    let mut url = url.to_string();
    if !url.contains("deezer.com") {
        url = follow(app, &url).await;
    }
    let re = Regex::new(r"deezer\.com/(?:[a-z]{2}(?:-[a-z]{2})?/)?(playlist|album|track)/(\d+)").unwrap();
    let c = re.captures(&url).ok_or("Unrecognised Deezer link")?;
    let (kind, id) = (c[1].to_string(), c[2].to_string());
    let api = |p: &str| format!("https://api.deezer.com/{p}");
    let check = |v: &Value| -> Result<(), String> {
        if v["error"].is_object() {
            Err(format!("Deezer: {}", s(&v["error"]["message"])))
        } else {
            Ok(())
        }
    };
    let map = |t: &Value, album: &str, art: &str| T {
        title: s(&t["title"]),
        artist: s(&t["artist"]["name"]),
        album: if album.is_empty() { s(&t["album"]["title"]) } else { album.into() },
        duration_ms: t["duration"].as_u64().unwrap_or(0) * 1000,
        artwork: if art.is_empty() { s(&t["album"]["cover_xl"]) } else { art.into() },
    };
    if kind == "track" {
        let t = get_json(app, &api(&format!("track/{id}")), &[]).await?;
        check(&t)?;
        let tr = map(&t, "", "");
        return Ok(L { source: "Deezer", title: tr.title.clone(), owner: tr.artist.clone(), tracks: vec![tr], note: String::new() });
    }
    let meta = get_json(app, &api(&format!("{kind}/{id}")), &[]).await?;
    check(&meta)?;
    let (album, art) = if kind == "album" { (s(&meta["title"]), s(&meta["cover_xl"])) } else { (String::new(), String::new()) };
    let mut tracks = vec![];
    let mut index = 0;
    loop {
        let page = get_json(app, &api(&format!("{kind}/{id}/tracks?index={index}&limit=100")), &[]).await?;
        check(&page)?;
        let data = page["data"].as_array().cloned().unwrap_or_default();
        if data.is_empty() {
            break;
        }
        index += data.len();
        tracks.extend(data.iter().map(|t| map(t, &album, &art)));
        if page["next"].is_null() || index >= 3000 {
            break;
        }
    }
    let owner = if kind == "album" { s(&meta["artist"]["name"]) } else { s(&meta["creator"]["name"]) };
    Ok(L { source: "Deezer", title: s(&meta["title"]), owner, tracks, note: String::new() })
}

// ---------------- Tidal (web API with the public web token) ----------------

const TIDAL_TOKENS: [&str; 2] = ["gsFXkJqGrUNoYMQPZe4k3WKwijnrp8iGSwn3bApe", "CzET4vdadNUFQ5JU"];

async fn tidal_get(app: &AppHandle, path: &str) -> Result<Value, String> {
    let cc = app.state::<AppState>().settings.lock().unwrap().country.clone();
    let sep = if path.contains('?') { '&' } else { '?' };
    let url = format!("https://api.tidal.com/v1/{path}{sep}countryCode={cc}");
    let mut last = "Tidal refused the request".to_string();
    for tok in TIDAL_TOKENS {
        let r = get(app, &url, &[("x-tidal-token", tok)]).await?;
        let status = r.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            continue;
        }
        let v: Value = r.json().await.map_err(|e| format!("unexpected Tidal response: {e}"))?;
        if v["status"].as_u64().map_or(false, |x| x >= 400) {
            last = format!("Tidal: {}", s(&v["userMessage"]));
            return Err(last);
        }
        return Ok(v);
    }
    Err(last)
}

fn tidal_track(t: &Value) -> T {
    let mut title = s(&t["title"]);
    if let Some(v) = t["version"].as_str().filter(|x| !x.is_empty()) {
        title = format!("{title} ({v})");
    }
    let artists: Vec<String> = t["artists"]
        .as_array()
        .map(|a| a.iter().map(|x| s(&x["name"])).collect())
        .unwrap_or_default();
    let cover = s(&t["album"]["cover"]);
    T {
        title,
        artist: if artists.is_empty() { s(&t["artist"]["name"]) } else { artists.join(", ") },
        album: s(&t["album"]["title"]),
        duration_ms: t["duration"].as_u64().unwrap_or(0) * 1000,
        artwork: if cover.is_empty() { String::new() } else { format!("https://resources.tidal.com/images/{}/1280x1280.jpg", cover.replace('-', "/")) },
    }
}

async fn tidal(app: &AppHandle, url: &str) -> Result<L, String> {
    let re = Regex::new(r"tidal\.com/(?:browse/)?(playlist|album|track)/([A-Za-z0-9-]+)").unwrap();
    let c = re.captures(url).ok_or("Unrecognised Tidal link")?;
    let (kind, id) = (c[1].to_string(), c[2].to_string());
    if kind == "track" {
        let t = tidal_get(app, &format!("tracks/{id}")).await?;
        let tr = tidal_track(&t);
        return Ok(L { source: "Tidal", title: tr.title.clone(), owner: tr.artist.clone(), tracks: vec![tr], note: String::new() });
    }
    let base = format!("{kind}s/{id}");
    let meta = tidal_get(app, &base).await?;
    let mut tracks = vec![];
    let mut offset = 0usize;
    loop {
        let page = tidal_get(app, &format!("{base}/items?limit=100&offset={offset}")).await?;
        let items = page["items"].as_array().cloned().unwrap_or_default();
        if items.is_empty() {
            break;
        }
        offset += items.len();
        for it in &items {
            if it["type"].as_str().map_or(true, |x| x == "track") {
                tracks.push(tidal_track(if it["item"].is_object() { &it["item"] } else { it }));
            }
        }
        if offset >= page["totalNumberOfItems"].as_u64().unwrap_or(0) as usize || offset >= 3000 {
            break;
        }
    }
    let owner = if kind == "album" { s(&meta["artist"]["name"]) } else { s(&meta["creator"]["name"]) };
    Ok(L { source: "Tidal", title: s(&meta["title"]), owner, tracks, note: String::new() })
}

// ---------------- Apple Music ----------------

fn from_catalog(t: catalog::Track) -> T {
    T { title: t.title, artist: t.artist, album: t.album, duration_ms: t.duration_ms, artwork: t.artwork }
}

async fn apple(app: &AppHandle, url: &str) -> Result<L, String> {
    let cc = Regex::new(r"music\.apple\.com/([a-z]{2})/")
        .unwrap()
        .captures(url)
        .map(|c| c[1].to_string())
        .unwrap_or_else(|| "us".into());
    let ids = |text: &str| -> Vec<u64> {
        Regex::new(r"(?:/|[?&]i=)(\d{6,})")
            .unwrap()
            .captures_iter(text)
            .filter_map(|c| c[1].parse().ok())
            .collect()
    };
    // Album/song links (and album?i=track) are plain iTunes catalog ids.
    if let Some(i) = Regex::new(r"[?&]i=(\d+)").unwrap().captures(url) {
        let tr = catalog::lookup_tracks(app, &[i[1].parse().unwrap_or(0)], &cc).await?;
        let t = from_catalog(tr.into_iter().next().ok_or("Song not found in this storefront")?);
        return Ok(L { source: "Apple Music", title: t.title.clone(), owner: t.artist.clone(), tracks: vec![t], note: String::new() });
    }
    if url.contains("/song/") {
        let id = ids(url).pop().ok_or("Unrecognised Apple Music link")?;
        let t = from_catalog(catalog::lookup_tracks(app, &[id], &cc).await?.into_iter().next().ok_or("Song not found")?);
        return Ok(L { source: "Apple Music", title: t.title.clone(), owner: t.artist.clone(), tracks: vec![t], note: String::new() });
    }
    if url.contains("/album/") {
        let id = ids(url).pop().ok_or("Unrecognised Apple Music link")?;
        let (rel, tracks) = catalog::fetch_release(app, id).await?;
        return Ok(L {
            source: "Apple Music",
            title: rel.name,
            owner: rel.artist,
            tracks: tracks.into_iter().map(from_catalog).collect(),
            note: String::new(),
        });
    }
    if !url.contains("/playlist/") {
        return Err("Only Apple Music playlists, albums and songs are supported.".into());
    }
    let html = get_text(app, url, &[]).await?;
    let re = Regex::new(r#"(?s)<script[^>]*type="application/ld\+json"[^>]*>(.*?)</script>"#).unwrap();
    let ld: Value = re
        .captures_iter(&html)
        .filter_map(|c| serde_json::from_str::<Value>(&c[1]).ok())
        .find(|v| v["track"].is_array())
        .ok_or("Could not read this Apple Music playlist (is it public?)")?;
    let list = ld["track"].as_array().cloned().unwrap_or_default();
    let mut found: std::collections::HashMap<u64, catalog::Track> = Default::default();
    let all: Vec<u64> = list.iter().filter_map(|t| ids(&s(&t["url"])).pop()).collect();
    for chunk in all.chunks(100) {
        for t in catalog::lookup_tracks(app, chunk, &cc).await.unwrap_or_default() {
            found.insert(t.id, t);
        }
    }
    let tracks = list
        .iter()
        .map(|t| match ids(&s(&t["url"])).pop().and_then(|id| found.get(&id)) {
            Some(f) => from_catalog(f.clone()),
            None => T { title: s(&t["name"]), ..Default::default() },
        })
        .collect();
    Ok(L {
        source: "Apple Music",
        title: s(&ld["name"]),
        owner: s(&ld["author"]["name"]),
        tracks,
        note: String::new(),
    })
}

// ---------------- Yandex Music (public API, best effort) ----------------

fn yandex_track(t: &Value) -> T {
    let mut title = s(&t["title"]);
    if let Some(v) = t["version"].as_str().filter(|x| !x.is_empty()) {
        title = format!("{title} ({v})");
    }
    let artists: Vec<String> = t["artists"]
        .as_array()
        .map(|a| a.iter().map(|x| s(&x["name"])).collect())
        .unwrap_or_default();
    let cover = if t["coverUri"].is_string() { s(&t["coverUri"]) } else { s(&t["albums"][0]["coverUri"]) };
    T {
        title,
        artist: artists.join(", "),
        album: s(&t["albums"][0]["title"]),
        duration_ms: t["durationMs"].as_u64().unwrap_or(0),
        artwork: if cover.is_empty() { String::new() } else { format!("https://{}", cover.replace("%%", "600x600")) },
    }
}

async fn yandex(app: &AppHandle, url: &str) -> Result<L, String> {
    const H: [(&str, &str); 2] = [("User-Agent", "Yandex-Music-API"), ("X-Yandex-Music-Client", "YandexMusicAndroid/24023231")];
    let api = |p: &str| format!("https://api.music.yandex.net/{p}");
    let fail = |v: &Value| -> Result<(), String> {
        if v["error"].is_object() {
            Err(format!(
                "Yandex Music refused the request ({}). It may be blocked in your region or the playlist is private.",
                s(&v["error"]["name"])
            ))
        } else {
            Ok(())
        }
    };
    let mut title = String::new();
    let mut owner = String::new();
    let mut tracks: Vec<T> = vec![];

    if let Some(c) = Regex::new(r"/users/([^/]+)/playlists/(\d+)").unwrap().captures(url) {
        let v = get_json(app, &api(&format!("users/{}/playlists/{}", &c[1], &c[2])), &H).await?;
        fail(&v)?;
        return yandex_playlist(app, &v["result"], &H).await;
    }
    if let Some(c) = Regex::new(r"/playlists/([A-Za-z0-9.-]+)").unwrap().captures(url) {
        let v = get_json(app, &api(&format!("playlist/{}", &c[1])), &H).await?;
        fail(&v)?;
        return yandex_playlist(app, &v["result"], &H).await;
    }
    if let Some(c) = Regex::new(r"/album/(\d+)(?:/track/(\d+))?").unwrap().captures(url) {
        if let Some(tid) = c.get(2) {
            let v = get_json(app, &api(&format!("tracks/{}", tid.as_str())), &H).await?;
            fail(&v)?;
            tracks = v["result"].as_array().map(|a| a.iter().map(yandex_track).collect()).unwrap_or_default();
        } else {
            let v = get_json(app, &api(&format!("albums/{}/with-tracks", &c[1])), &H).await?;
            fail(&v)?;
            let r = &v["result"];
            title = s(&r["title"]);
            owner = r["artists"].as_array().map(|a| a.iter().map(|x| s(&x["name"])).collect::<Vec<_>>().join(", ")).unwrap_or_default();
            for vol in r["volumes"].as_array().cloned().unwrap_or_default() {
                tracks.extend(vol.as_array().cloned().unwrap_or_default().iter().map(yandex_track));
            }
            let art = if r["coverUri"].is_string() { format!("https://{}", s(&r["coverUri"]).replace("%%", "600x600")) } else { String::new() };
            for t in &mut tracks {
                if t.album.is_empty() {
                    t.album = title.clone();
                }
                if t.artwork.is_empty() {
                    t.artwork = art.clone();
                }
            }
        }
    } else if let Some(c) = Regex::new(r"/track/(\d+)").unwrap().captures(url) {
        let v = get_json(app, &api(&format!("tracks/{}", &c[1])), &H).await?;
        fail(&v)?;
        tracks = v["result"].as_array().map(|a| a.iter().map(yandex_track).collect()).unwrap_or_default();
    } else {
        return Err("Unrecognised Yandex Music link (playlist, album or track expected).".into());
    }
    if title.is_empty() {
        title = tracks.first().map(|t| t.title.clone()).unwrap_or_default();
        owner = tracks.first().map(|t| t.artist.clone()).unwrap_or_default();
    }
    Ok(L { source: "Yandex Music", title, owner, tracks, note: String::new() })
}

async fn yandex_playlist(app: &AppHandle, r: &Value, h: &[(&str, &str); 2]) -> Result<L, String> {
    let items = r["tracks"].as_array().cloned().unwrap_or_default();
    let mut tracks = vec![];
    let mut stubs: Vec<String> = vec![];
    for it in &items {
        if it["track"].is_object() {
            tracks.push(yandex_track(&it["track"]));
        } else if let Some(id) = it["id"].as_u64().map(|x| x.to_string()).or_else(|| it["id"].as_str().map(String::from)) {
            stubs.push(match it["albumId"].as_u64() {
                Some(a) => format!("{id}:{a}"),
                None => id,
            });
        }
    }
    // Stubs (id only) need a second request to get titles and artists.
    let http = app.state::<AppState>().http.clone();
    for chunk in stubs.chunks(100) {
        let resp = http
            .post("https://api.music.yandex.net/tracks")
            .header("User-Agent", h[0].1)
            .header(h[1].0, h[1].1)
            .form(&[("track-ids", chunk.join(","))])
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let v: Value = resp.json().await.map_err(|e| e.to_string())?;
        tracks.extend(v["result"].as_array().map(|a| a.iter().map(yandex_track).collect::<Vec<_>>()).unwrap_or_default());
    }
    Ok(L {
        source: "Yandex Music",
        title: s(&r["title"]),
        owner: if r["owner"]["name"].is_string() { s(&r["owner"]["name"]) } else { s(&r["owner"]["login"]) },
        tracks,
        note: String::new(),
    })
}

// ---------------- dispatcher ----------------

/// Returns `Ok(None)` for links that aren't a known streaming service (yt-dlp handles those).
#[tauri::command]
pub async fn import_playlist(app: AppHandle, url: String) -> Result<Option<Value>, String> {
    let url = url.trim().to_string();
    let host = reqwest::Url::parse(&url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_lowercase()))
        .unwrap_or_default();
    let is = |d: &str| host == d || host.ends_with(&format!(".{d}"));

    let list = if is("spotify.com") || is("spotify.link") {
        spotify(&app, &url).await?
    } else if is("deezer.com") || is("deezer.page.link") || is("dzr.page.link") {
        deezer(&app, &url).await?
    } else if is("tidal.com") {
        tidal(&app, &url).await?
    } else if host == "music.apple.com" || host == "itunes.apple.com" {
        apple(&app, &url).await?
    } else if host.starts_with("music.yandex.") || host.starts_with("music.ya") {
        yandex(&app, &url).await?
    } else if is("zvuk.com") || is("sber-zvuk.com") || is("sberzvuk.com") || is("boom.ru") || is("vk.com") && url.contains("music") || is("vk.ru") && url.contains("music") || host.starts_with("music.amazon.") {
        return Err("This service doesn't expose playlists without logging in, so the app can't read it. Copy the track list and paste it here instead (one \"Artist - Title\" per line).".into());
    } else {
        return Ok(None);
    };

    if list.tracks.is_empty() {
        return Err("No tracks found (is the playlist public?)".into());
    }
    let entries: Vec<Value> = list
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            json!({
                "id": i.to_string(),
                "title": t.title,
                "url": "",
                "duration": if t.duration_ms > 0 { json!(t.duration_ms as f64 / 1000.0) } else { Value::Null },
                "uploader": t.artist,
                "album": t.album,
                "thumbnail": t.artwork,
                "untitled": false,
                "imported": true,
            })
        })
        .collect();
    Ok(Some(json!({
        "url": url,
        "title": list.title,
        "uploader": list.owner,
        "is_playlist": true,
        "extractor": list.source,
        "note": list.note,
        "entries": entries,
    })))
}

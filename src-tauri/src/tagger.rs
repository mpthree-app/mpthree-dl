use lofty::{
    config::WriteOptions,
    picture::{MimeType, Picture, PictureType},
    prelude::*,
    probe::Probe,
    tag::{ItemKey, Tag},
};
use std::path::Path;
use tauri::{AppHandle, Manager};

use crate::{catalog, queue::Item, AppState};

/// Cover art bytes (high-res first), cached per artwork URL so an album's tracks share one fetch.
pub async fn cover(app: &AppHandle, base: &str) -> Option<Vec<u8>> {
    if base.is_empty() {
        return None;
    }
    let st = app.state::<AppState>();
    if let Some(b) = st.covers.lock().unwrap().get(base) {
        return Some(b.clone());
    }
    for size in [1400u32, 600] {
        if let Ok(r) = st.http.get(catalog::art(base, size)).send().await {
            if r.status().is_success() {
                if let Ok(b) = r.bytes().await {
                    let v = b.to_vec();
                    let mut c = st.covers.lock().unwrap();
                    if c.len() >= 24 {
                        c.clear();
                    }
                    c.insert(base.to_string(), v.clone());
                    return Some(v);
                }
            }
        }
    }
    None
}

pub fn write_tags(path: &Path, it: &Item, cover: Option<&[u8]>) -> Result<(), String> {
    let mut f = Probe::open(path)
        .map_err(|e| e.to_string())?
        .guess_file_type()
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| e.to_string())?;
    if f.primary_tag().is_none() {
        let tt = f.primary_tag_type();
        f.insert_tag(Tag::new(tt));
    }
    let tag = f.primary_tag_mut().ok_or("no writable tag")?;

    tag.set_title(it.title.clone());
    tag.set_artist(it.artist.clone());
    if !it.album.is_empty() {
        tag.set_album(it.album.clone());
    }
    let aa = if it.album_artist.is_empty() { &it.artist } else { &it.album_artist };
    tag.insert_text(ItemKey::AlbumArtist, aa.clone());
    if it.track_no > 0 {
        tag.set_track(it.track_no);
        if it.track_total > 0 {
            tag.set_track_total(it.track_total);
        }
    }
    if it.disc_no > 0 {
        tag.set_disk(it.disc_no);
        if it.disc_total > 0 {
            tag.set_disk_total(it.disc_total);
        }
    }
    if let Ok(y) = it.year.parse::<u32>() {
        tag.set_year(y);
    }
    if !it.genre.is_empty() {
        tag.set_genre(it.genre.clone());
    }
    if let Some(b) = cover {
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(Picture::new_unchecked(
            PictureType::CoverFront,
            Some(MimeType::Jpeg),
            None,
            b.to_vec(),
        ));
    }
    tag.save_to_path(path, WriteOptions::default())
        .map_err(|e| e.to_string())
}

# mpthree-dl

A small black desktop app (Tauri 2) for downloading music with [yt-dlp](https://github.com/yt-dlp/yt-dlp), with proper tags and cover art.

## Disclaimer / Отказ от ответственности

**English.** mpthree-dl is provided **for personal use and educational purposes only**. You may use it only with content you have the legal right to download and keep (your own files, public-domain or freely licensed music, material you are permitted to copy). The author does **not** encourage or condone copyright infringement or any violation of the terms of service of third-party websites. The software is provided "as is", without warranty of any kind. The author takes **no risk and no responsibility** for how the app is used or for any consequences of its use, including legal claims, account restrictions, data loss or damage. You use it entirely at your own risk and are solely responsible for complying with the laws of your country and the rules of the services you access.

**Русский.** mpthree-dl предоставляется **исключительно для личного использования и в образовательных целях**. Используйте приложение только с тем контентом, который вы имеете законное право скачивать и хранить (ваши собственные файлы, музыка в общественном достоянии или со свободной лицензией, материалы, копирование которых вам разрешено). Автор **не поощряет** и не одобряет нарушение авторских прав или условий использования сторонних сервисов. Программа предоставляется «как есть», без каких-либо гарантий. Автор **не несёт никаких рисков и никакой ответственности** за то, как используется приложение, и за любые последствия его использования, включая юридические претензии, ограничения аккаунтов, потерю данных или ущерб. Вы используете приложение полностью на свой страх и риск и самостоятельно отвечаете за соблюдение законов вашей страны и правил сервисов, к которым обращаетесь.

## What it does

- **Search** artists, releases and songs in a music catalog: **iTunes** (covers, track lists, years, genres, 30 s previews) or **Deezer** (public API, no login; full discographies with covers and previews). You can also search **YouTube**, **YouTube Music** or **SoundCloud** directly.
- **Preview** any catalog track (30 s clip) or any web result (full stream via yt-dlp) before downloading.
- **Download a full release** (album, EP or single) or an artist's **entire discography**. Filter by albums / EPs / singles, untick what you don't want, and the app keeps one edition per release (no deluxe/remaster duplicates).
- **Import playlists from streaming services**: paste a **Spotify**, **Deezer**, **Apple Music**, **Tidal** or **Yandex Music** playlist, album or track link. Only metadata is read (title, artist, album, cover); each track is then matched to audio on YouTube / SoundCloud when downloaded, and tagged. Playlists of any size are read in full without login (Spotify is paged 100 at a time through the web player's anonymous token; tested with 1,200+ tracks). Zvuk, VK Music and other login-only services can't be read: paste a track list instead (one `Artist - Title` per line).
- **Import by link**: paste one or more URLs: playlists, albums, single tracks from YouTube, SoundCloud, Bandcamp or any of the [1800+ sites yt-dlp supports](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md). Choose which entries to download.
- **Quality** selector: Best (original stream, no re-encode), FLAC, WAV, MP3 320 / V0 / 128, M4A (AAC), Opus.
- **Metadata**: catalog downloads are tagged with title, artist, album, album artist, track/disc numbers, year, genre and 1400 px cover art (MP3, FLAC, M4A, Opus, WAV). Link downloads use yt-dlp's embedded metadata and thumbnail, with `Artist - Title` split out of video titles.
- Parallel download queue with progress, cancel, manual retry and **automatic retry** (default 3 attempts, configurable; a retry for a catalog/imported track moves on to the next-best source). Files are organised as `Artist/Album (Year)/01 - Title.ext`.

### How catalog downloads find audio

The catalog only supplies metadata. For each track the app searches YouTube (then SoundCloud) through yt-dlp, scores the candidates by duration, title/artist match and "Topic" auto-generated channels, penalises live/remix/cover/slowed versions, and downloads the best match. Quality is bounded by the source: YouTube audio is lossy (Opus ~130-160 kbps / AAC), so FLAC/WAV from YouTube is a larger container, not better sound. Use Bandcamp or other lossless sources via **Link** when you need real lossless.

## Requirements

- Rust, Node.js, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) (WebView2 + MSVC build tools on Windows).
- yt-dlp and ffmpeg. If they're not on `PATH`, open **Settings → Tools** and the app installs its own copies (Windows) into its local data folder.

## Run / build

```bash
npm install
npm run dev      # development
npm run build    # installer in src-tauri/target/release/bundle
```

## Settings

Download folder, folder organisation, parallel downloads, catalog store country, cookies-from-browser (for age-restricted / login-only content), and extra yt-dlp arguments.

## Notes

- Spotify / Apple Music / Tidal links aren't supported by yt-dlp (DRM). Search for the same artist or release in the catalog instead.
- Only download music you have the right to download (see the disclaimer above).

# mpthree-dl

A small black desktop app (Tauri 2) for downloading music with [yt-dlp](https://github.com/yt-dlp/yt-dlp), with proper tags and cover art.

## Disclaimer / Отказ от ответственности

**English.** mpthree-dl is intended for **personal use and educational purposes**, with content you have the right to download and keep (your own files, public-domain or freely licensed music, material you are permitted to copy). You are responsible for having the necessary rights and permissions for what you download and for following the laws that apply to you and the terms of the services you access; this statement of intended use does not by itself make any particular use lawful. The author does not encourage copyright infringement or violation of third-party terms of service. The software is provided "as is", without warranty, and to the maximum extent permitted by applicable law the author and contributors are not liable for indirect or consequential damages, data loss or account restrictions arising from its use; liability that cannot be excluded by law is not affected. mpthree-dl is not affiliated with, endorsed by or sponsored by any of the services it works with. Full text: [Terms](https://mpthreedl.vercel.app/terms), [Privacy](https://mpthreedl.vercel.app/privacy), [Copyright / DMCA](https://mpthreedl.vercel.app/copyright).

**Русский.** mpthree-dl предназначен для **личного использования и образовательных целей**, с контентом, который вы вправе скачивать и хранить (ваши файлы, музыка в общественном достоянии или со свободной лицензией, материалы, копирование которых вам разрешено). Вы отвечаете за наличие необходимых прав и разрешений на скачиваемое и за соблюдение применимых к вам законов и условий сервисов, к которым обращаетесь; это заявление о предполагаемом использовании само по себе не делает законным какое-либо конкретное использование. Автор не поощряет нарушение авторских прав или условий сторонних сервисов. Программа предоставляется «как есть», без гарантий; в максимально допустимой применимым правом степени автор и участники не отвечают за косвенные и последовавшие убытки, потерю данных и ограничения аккаунтов, возникшие из-за её использования; ответственность, которую нельзя исключить по закону, не затрагивается. mpthree-dl не связан с сервисами, с которыми работает, и не одобрен и не спонсируется ими. Полные тексты: [Условия](https://mpthreedl.vercel.app/terms), [Конфиденциальность](https://mpthreedl.vercel.app/privacy), [Авторские права / DMCA](https://mpthreedl.vercel.app/copyright).

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

- Audio from Spotify / Apple Music / Tidal can't be downloaded by yt-dlp (DRM). Importing their links only reads metadata (title, artist, album, cover); the audio is then matched on YouTube / SoundCloud. You can also search the same artist or release in the catalog.
- Only download music you have the right to download (see the disclaimer above).

## Network and data

Settings and the download queue (track details and source links) are stored locally in the app's config folder; downloaded files go to the folder you choose. The app has no server of its own. It sends requests from your device directly to third parties depending on what you do: iTunes Search and Deezer (catalog), Spotify, Deezer, Tidal and Yandex Music web/app endpoints (link import), YouTube, YouTube Music, SoundCloud and other sites through yt-dlp, and GitHub (installing yt-dlp / ffmpeg from Settings → Tools). Those services see your IP address and request details under their own policies. If you set "cookies from browser", yt-dlp reads cookies from that browser profile on your device and uses them for requests to the sites you access. The app shows a banner for mpthree.fun; the site opens only if you click it. No analytics or telemetry code was found in this repository. See the [Privacy Policy](https://mpthreedl.vercel.app/privacy).

## Third-party software and license

mpthree-dl's own code is under the MIT License (see [LICENSE](LICENSE)). It does not change the licenses of third-party software: yt-dlp and ffmpeg are not bundled but can be downloaded by the app on request and keep their own licenses (the ffmpeg build used is labelled "gpl"); the app also uses Tauri and other Rust crates (see `src-tauri/Cargo.toml`) and the Inter and Dela Gothic One fonts. Service names and trademarks belong to their owners.

## Copyright and contact

This repository contains source code and documentation only, no music. Concerns about the code, docs or website: open an [issue](https://github.com/mpthree-app/mpthree-dl/issues) (public) or notify GitHub under its DMCA policy. Complaints about content on YouTube, SoundCloud or other services go to those services. See [Copyright / DMCA](https://mpthreedl.vercel.app/copyright).

"use strict";
// Add a language by adding a dictionary here and an entry to LANGS.
const LANGS = [["en", "English"], ["ru", "Русский"]];

const DICT = {
  en: {
    "nav.search": "Search", "nav.link": "Link", "nav.queue": "Queue", "nav.settings": "Settings",
    "quality": "Quality", "quality.tip": "Output format for every download", "language": "Language",
    "q.best": "Best (original stream)", "q.short.best": "Best",

    "search.ph": "Search artists, releases, songs…", "search.go": "Search",
    "kind.artists": "Artists", "kind.releases": "Releases", "kind.songs": "Songs",
    "search.searching": "Searching…", "search.none": "No results.",
    "hero.title": "Find music to download",
    "hero.text": "Search artists, releases and songs, or switch the source to Deezer, YouTube, YouTube Music or SoundCloud.<br>Open an artist to grab a whole discography, or a release to preview tracks first.",

    "back": "← Back", "tip.preview": "Preview", "tip.download": "Download",
    "kind.album": "Album", "kind.ep": "EP", "kind.single": "Single",
    "artist": "Artist", "artist.loading": "Loading discography…", "release.loading": "Loading tracks…",
    "artist.count": "{n} releases in catalog",
    "artist.dl": "Download selected: {n}", "artist.dl.tracks": " (~{t} tracks)",
    "sel.all": "Select all", "sel.none": "None",
    "f.albums": "Albums", "f.eps": "EPs", "f.singles": "Singles",
    "f.primary": "Only releases credited to {name}", "f.dedupe": "One edition per release (hides deluxe, remaster…)",
    "no.match": "No releases match.", "reading.releases": "Reading {n} release(s)…",
    "release.dl": "Download {kind}", "release.previews": "Previews are 30-second clips",
    "release.meta": "{n} tracks · {m} min",
    "preview.loading": "loading…", "preview.failed": "Preview failed: ", "preview.none": "No preview available",
    "preview.30": "30s preview",

    "link.ph": "Paste links: Spotify, Deezer, Apple Music, Tidal, Yandex Music, YouTube, SoundCloud, Bandcamp… or a track list (one “Artist - Title” per line)",
    "link.fetch": "Fetch", "link.number": "Prefix playlist tracks with their number",
    "link.paste": "Paste a link or a track list", "link.reading": "Reading link {i} of {n}…",
    "link.tracks": "{n} tracks", "link.single": "single track",
    "link.meta": "Metadata only: each track is matched to audio on YouTube / SoundCloud when downloaded.",
    "link.dl": "Download {n} selected", "link.as": "as {q}",
    "link.tracklist": "Track list", "link.pasted": "Pasted list",

    "queue.open": "Open folder", "queue.cancelAll": "Cancel all", "queue.clear": "Clear finished",
    "queue.empty": "Nothing in the queue.", "queue.summary": "{a} active · {d} done", "queue.failed": " · {f} failed",
    "st.queued": "Queued", "st.resolving": "Finding source…", "st.downloading": "Downloading", "st.converting": "Converting",
    "st.tagging": "Tagging", "st.done": "Done", "st.error": "Failed", "st.cancelled": "Cancelled",
    "tip.cancel": "Cancel", "tip.retry": "Retry", "tip.reveal": "Show in folder",
    "toast.added": "Added {n} to queue", "toast.already": "Already in the queue",

    "s.dir": "Download folder", "s.browse": "Browse", "s.open": "Open",
    "s.org": "Organize into Artist / Album folders", "s.skip": "Skip tracks already in the download folder",
    "s.scan": "Scan library", "s.scanning": "Scanning…", "s.found": "{n} audio files found",
    "s.tray": "Keep running in the system tray when the window is closed",
    "s.conc": "Parallel downloads", "s.country": "Catalog store country", "s.retries": "Auto-retry failed downloads",
    "s.cookies": "Use cookies from browser", "s.cookies.hint": "(age-restricted or login-only content)", "s.none": "None",
    "s.extra": "Extra yt-dlp arguments",
    "s.tools": "Tools", "s.install": "Install", "s.update": "Update", "s.working": "Working…", "s.downloading": "Downloading {pct}%",
    "s.notfound": "Not found", "s.ffmpeg.missing": "Not found (needed for conversion and tagging)",
    "s.aboutq": "About quality",
    "s.aboutq.text": "Audio is pulled from the best source yt-dlp can find. <b>Best</b> keeps the original stream untouched (usually Opus or AAC on YouTube). FLAC and WAV are lossless containers, but they cannot add detail the source never had; they only make sense for lossless sources such as Bandcamp. Catalog tracks are matched to a source by title, artist and duration, then tagged with catalog metadata and cover art.",
    "banner.text": "{tools} not found.", "banner.link": "Open Settings → Tools", "banner.tail": "to install.",

    "ob.title": "What do you want to download?",
    "ob.sub": "Pick an artist to grab their discography, or paste a link to a playlist, album or track.",
    "ob.artist": "Pick an artist", "ob.link": "Paste a link",
    "ob.q": "Start typing an artist name…", "ob.noartists": "No artists found.",
    "ob.in": "Spotify, Deezer, Apple Music, Tidal, Yandex Music, YouTube, SoundCloud, Bandcamp…\nor a track list, one “Artist - Title” per line",
    "ob.go": "Fetch tracks", "ob.saving": "Saving to", "ob.change": "Change", "ob.skip": "Skip for now",
    "ob.welcome.title": "Download your music", "ob.welcome.sub": "Search artists, paste a playlist link, and get tagged files with cover art, in the format you choose.",
    "ob.start": "Get started", "ob.back": "Back",

    "ad.badge": "Ad", "ad.title": "mpthree.fun",
    "ad.text": "Your downloads deserve a proper player. Local music player — your library, offline, no streaming.",
    "ad.cta": "Try it free", "ad.hide": "Hide",
  },

  ru: {
    "nav.search": "Поиск", "nav.link": "Ссылка", "nav.queue": "Очередь", "nav.settings": "Настройки",
    "quality": "Качество", "quality.tip": "Формат файла для всех загрузок", "language": "Язык",
    "q.best": "Лучшее (оригинальный поток)", "q.short.best": "Лучшее",

    "search.ph": "Артисты, релизы, песни…", "search.go": "Найти",
    "kind.artists": "Артисты", "kind.releases": "Релизы", "kind.songs": "Песни",
    "search.searching": "Ищем…", "search.none": "Ничего не найдено.",
    "hero.title": "Найдите музыку для загрузки",
    "hero.text": "Ищите артистов, релизы и песни или переключите источник на Deezer, YouTube, YouTube Music или SoundCloud.<br>Откройте артиста, чтобы скачать всю дискографию, или релиз, чтобы сначала послушать превью.",

    "back": "← Назад", "tip.preview": "Прослушать", "tip.download": "Скачать",
    "kind.album": "Альбом", "kind.ep": "EP", "kind.single": "Сингл",
    "artist": "Артист", "artist.loading": "Загружаем дискографию…", "release.loading": "Загружаем треки…",
    "artist.count": "Релизов в каталоге: {n}",
    "artist.dl": "Скачать выбранное: {n}", "artist.dl.tracks": " (≈{t} треков)",
    "sel.all": "Выбрать все", "sel.none": "Снять",
    "f.albums": "Альбомы", "f.eps": "EP", "f.singles": "Синглы",
    "f.primary": "Только релизы {name}", "f.dedupe": "Одно издание релиза (без deluxe, ремастеров…)",
    "no.match": "Подходящих релизов нет.", "reading.releases": "Читаем релизы: {n}…",
    "release.dl": "Скачать ({kind})", "release.previews": "Превью — 30 секунд",
    "release.meta": "Треков: {n} · {m} мин",
    "preview.loading": "загрузка…", "preview.failed": "Не удалось воспроизвести: ", "preview.none": "Превью недоступно",
    "preview.30": "превью 30 с",

    "link.ph": "Вставьте ссылки: Spotify, Deezer, Apple Music, Tidal, Яндекс Музыка, YouTube, SoundCloud, Bandcamp… или список треков (по одному «Артист - Название» в строке)",
    "link.fetch": "Загрузить", "link.number": "Нумеровать треки плейлиста в названии",
    "link.paste": "Вставьте ссылку или список треков", "link.reading": "Читаем ссылку {i} из {n}…",
    "link.tracks": "треков: {n}", "link.single": "один трек",
    "link.meta": "Только метаданные: при загрузке каждый трек ищется на YouTube / SoundCloud.",
    "link.dl": "Скачать выбранное: {n}", "link.as": "в формате {q}",
    "link.tracklist": "Список треков", "link.pasted": "Вставленный список",

    "queue.open": "Открыть папку", "queue.cancelAll": "Отменить все", "queue.clear": "Убрать завершённые",
    "queue.empty": "В очереди пусто.", "queue.summary": "Активно: {a} · готово: {d}", "queue.failed": " · ошибок: {f}",
    "st.queued": "В очереди", "st.resolving": "Ищем источник…", "st.downloading": "Загрузка", "st.converting": "Конвертация",
    "st.tagging": "Теги", "st.done": "Готово", "st.error": "Ошибка", "st.cancelled": "Отменено",
    "tip.cancel": "Отменить", "tip.retry": "Повторить", "tip.reveal": "Показать в папке",
    "toast.added": "Добавлено в очередь: {n}", "toast.already": "Уже в очереди",

    "s.dir": "Папка загрузок", "s.browse": "Обзор", "s.open": "Открыть",
    "s.org": "Раскладывать по папкам Артист / Альбом", "s.skip": "Пропускать треки, которые уже есть в папке",
    "s.scan": "Сканировать библиотеку", "s.scanning": "Сканируем…", "s.found": "Найдено аудиофайлов: {n}",
    "s.tray": "Продолжать работу в трее после закрытия окна",
    "s.conc": "Параллельных загрузок", "s.country": "Страна каталога", "s.retries": "Автоповтор при ошибке",
    "s.cookies": "Cookies из браузера", "s.cookies.hint": "(для контента с возрастным ограничением или входом)", "s.none": "Нет",
    "s.extra": "Доп. аргументы yt-dlp",
    "s.tools": "Инструменты", "s.install": "Установить", "s.update": "Обновить", "s.working": "Работаем…", "s.downloading": "Загрузка {pct}%",
    "s.notfound": "Не найден", "s.ffmpeg.missing": "Не найден (нужен для конвертации и тегов)",
    "s.aboutq": "О качестве",
    "s.aboutq.text": "Звук берётся из лучшего источника, который найдёт yt-dlp. <b>Лучшее</b> сохраняет оригинальный поток без изменений (на YouTube обычно Opus или AAC). FLAC и WAV — контейнеры без потерь, но они не добавят деталей, которых нет в источнике; смысл в них есть только для lossless-источников вроде Bandcamp. Треки из каталога подбираются по названию, артисту и длительности, затем получают теги и обложку из каталога.",
    "banner.text": "Не найдено: {tools}.", "banner.link": "Откройте Настройки → Инструменты", "banner.tail": "для установки.",

    "ob.title": "Что хотите скачать?",
    "ob.sub": "Выберите артиста, чтобы скачать его дискографию, или вставьте ссылку на плейлист, альбом или трек.",
    "ob.artist": "Выбрать артиста", "ob.link": "Вставить ссылку",
    "ob.q": "Начните вводить имя артиста…", "ob.noartists": "Артисты не найдены.",
    "ob.in": "Spotify, Deezer, Apple Music, Tidal, Яндекс Музыка, YouTube, SoundCloud, Bandcamp…\nили список треков, по одному «Артист - Название» в строке",
    "ob.go": "Загрузить треки", "ob.saving": "Сохраняем в", "ob.change": "Изменить", "ob.skip": "Пропустить",
    "ob.welcome.title": "Скачивайте свою музыку", "ob.welcome.sub": "Ищите артистов, вставляйте ссылки на плейлисты и получайте файлы с тегами и обложками в нужном формате.",
    "ob.start": "Начать", "ob.back": "Назад",

    "ad.badge": "Реклама", "ad.title": "mpthree.fun",
    "ad.text": "Скачанной музыке нужен хороший плеер. Локальный музыкальный плеер — ваша библиотека, офлайн, без стриминга.",
    "ad.cta": "Попробовать", "ad.hide": "Скрыть",
  },
};

let LANG = "en";
function t(key, vars) {
  let s = DICT[LANG]?.[key] ?? DICT.en[key] ?? key;
  if (vars) for (const k in vars) s = s.split(`{${k}}`).join(vars[k]);
  return s;
}
function detectLang() {
  const n = (navigator.language || "en").slice(0, 2).toLowerCase();
  return DICT[n] ? n : "en";
}
function applyI18n() {
  document.documentElement.lang = LANG;
  for (const el of document.querySelectorAll("[data-i18n]")) el.textContent = t(el.dataset.i18n);
  for (const el of document.querySelectorAll("[data-i18n-html]")) el.innerHTML = t(el.dataset.i18nHtml);
  for (const el of document.querySelectorAll("[data-i18n-ph]")) el.placeholder = t(el.dataset.i18nPh);
  for (const el of document.querySelectorAll("[data-i18n-title]")) el.title = t(el.dataset.i18nTitle);
}

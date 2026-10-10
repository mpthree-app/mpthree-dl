"use strict";
// Add a language by adding a dictionary here and an entry to LANGS.
const LANGS = [["en", "English"], ["ru", "Русский"]];

const DICT = {
  en: {
    "src.cat": "Catalog", "src.cat2": "Catalog 2", "src.video": "Web video", "src.music": "Web music", "src.audio": "Web audio",
    "nav.search": "Search", "nav.link": "Link", "nav.queue": "Queue", "nav.settings": "Settings",
    "quality": "Quality", "quality.tip": "Output format for every download", "language": "Language",
    "q.best": "Best (original stream)", "q.short.best": "Best",

    "search.ph": "Search artists, releases, songs…", "search.go": "Search",
    "kind.artists": "Artists", "kind.releases": "Releases", "kind.songs": "Songs",
    "search.searching": "Searching…", "search.none": "No results.",
    "hero.title": "Find music to download",
    "hero.text": "Search artists, releases and songs, or switch the source to another catalog or a web source.<br>Open an artist to grab a whole discography, or a release to preview tracks first.",

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

    "link.ph": "Paste links to playlists, albums or tracks… or a track list (one “Artist - Title” per line)",
    "link.fetch": "Fetch", "link.number": "Prefix playlist tracks with their number",
    "link.paste": "Paste a link or a track list", "link.reading": "Reading link {i} of {n}…",
    "link.tracks": "{n} tracks", "link.single": "single track",
    "link.meta": "Metadata only: each track is matched to audio online when downloaded.",
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
    "s.aboutq.text": "Audio is pulled from the best source yt-dlp can find. <b>Best</b> keeps the original stream untouched (usually Opus or AAC). FLAC and WAV are lossless containers, but they cannot add detail the source never had; they only make sense for lossless sources. Catalog tracks are matched to a source by title, artist and duration, then tagged with catalog metadata and cover art.",
    "banner.text": "{tools} not found.", "banner.link": "Open Settings → Tools", "banner.tail": "to install.",

    "ob.title": "What do you want to download?",
    "ob.sub": "Pick an artist to grab their discography, or paste a link to a playlist, album or track.",
    "ob.artist": "Pick an artist", "ob.link": "Paste a link",
    "ob.q": "Start typing an artist name…", "ob.noartists": "No artists found.",
    "ob.in": "Playlist, album or track links…\nor a track list, one “Artist - Title” per line",
    "ob.go": "Fetch tracks", "ob.saving": "Saving to", "ob.change": "Change", "ob.skip": "Skip for now",
    "ob.welcome.title": "Download your music", "ob.welcome.sub": "Search artists, paste a playlist link, and get tagged files with cover art, in the format you choose.",
    "ob.start": "Get started", "ob.back": "Back",
    "ob.f1.t": "Find", "ob.f1.d": "Artists, albums, songs",
    "ob.f2.t": "Paste", "ob.f2.d": "Playlists from any service",
    "ob.f3.t": "Tag", "ob.f3.d": "Covers & metadata built in",
    "ob.disc": "mpthree-dl is a tool for personal use. You are responsible for what you download and must respect copyright law and the terms of the services you use. We don't host or distribute any music.",
    "ob.agree": "I have read and agree to the", "ob.and": "and", "ob.continue": "Continue",
    "doc.terms": "Terms of Use", "doc.privacy": "Privacy Policy", "doc.copyright": "Copyright & DMCA",

    "s.filter": "Search settings", "s.nomatch": "No matches", "s.view": "Open", "s.legal": "Legal",
    "s.g.general": "General", "s.g.advanced": "Advanced", "s.g.other": "Other",
    "s.p.downloads": "Downloads", "s.p.system": "System", "s.p.engine": "Engine", "s.p.tools": "Tools", "s.p.about": "About",
    "s.files": "Files", "s.library": "Library", "s.perf": "Performance", "s.net": "Network & sources",
    "s.dir.hint": "Where finished tracks are saved", "s.org.hint": "Creates Artist / Album subfolders",
    "s.skip.hint": "Never downloads the same track twice", "s.scan.hint": "Counts audio files in the download folder",
    "s.tray.t": "Close to tray", "s.lang.hint": "Interface language",
    "s.conc.hint": "How many tracks download at once", "s.retries.hint": "Attempts before a track is marked as failed",
    "s.country.hint": "Two-letter store code used for catalog search", "s.cookies.h": "For age-restricted or login-only content",
    "s.extra.hint": "Passed straight to yt-dlp", "s.tools.hint": "Helpers mpthree-dl needs; installed on demand",
    "s.legal.terms": "The rules for using mpthree-dl", "s.legal.privacy": "What stays on your device", "s.legal.copyright": "Respecting rights holders",
    "win.min": "Minimize", "win.max": "Maximize", "win.restore": "Restore", "win.close": "Close",
    "ad.title": "mpthree.fun",
    "ad.text": "A local music player that can do everything.",
  },

  ru: {
    "src.cat": "Каталог", "src.cat2": "Каталог 2", "src.video": "Веб-видео", "src.music": "Веб-музыка", "src.audio": "Веб-аудио",
    "nav.search": "Поиск", "nav.link": "Ссылка", "nav.queue": "Очередь", "nav.settings": "Настройки",
    "quality": "Качество", "quality.tip": "Формат файла для всех загрузок", "language": "Язык",
    "q.best": "Лучшее (оригинальный поток)", "q.short.best": "Лучшее",

    "search.ph": "Артисты, релизы, песни…", "search.go": "Найти",
    "kind.artists": "Артисты", "kind.releases": "Релизы", "kind.songs": "Песни",
    "search.searching": "Ищем…", "search.none": "Ничего не найдено.",
    "hero.title": "Найдите музыку для загрузки",
    "hero.text": "Ищите артистов, релизы и песни или переключите источник на другой каталог или веб-источник.<br>Откройте артиста, чтобы скачать всю дискографию, или релиз, чтобы сначала послушать превью.",

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

    "link.ph": "Вставьте ссылки на плейлисты, альбомы или треки… или список треков (по одному «Артист - Название» в строке)",
    "link.fetch": "Загрузить", "link.number": "Нумеровать треки плейлиста в названии",
    "link.paste": "Вставьте ссылку или список треков", "link.reading": "Читаем ссылку {i} из {n}…",
    "link.tracks": "треков: {n}", "link.single": "один трек",
    "link.meta": "Только метаданные: при загрузке для каждого трека ищется аудио в сети.",
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
    "s.aboutq.text": "Звук берётся из лучшего источника, который найдёт yt-dlp. <b>Лучшее</b> сохраняет оригинальный поток без изменений (обычно Opus или AAC). FLAC и WAV — контейнеры без потерь, но они не добавят деталей, которых нет в источнике; смысл в них есть только для lossless-источников. Треки из каталога подбираются по названию, артисту и длительности, затем получают теги и обложку из каталога.",
    "banner.text": "Не найдено: {tools}.", "banner.link": "Откройте Настройки → Инструменты", "banner.tail": "для установки.",

    "ob.title": "Что хотите скачать?",
    "ob.sub": "Выберите артиста, чтобы скачать его дискографию, или вставьте ссылку на плейлист, альбом или трек.",
    "ob.artist": "Выбрать артиста", "ob.link": "Вставить ссылку",
    "ob.q": "Начните вводить имя артиста…", "ob.noartists": "Артисты не найдены.",
    "ob.in": "Ссылки на плейлисты, альбомы или треки…\nили список треков, по одному «Артист - Название» в строке",
    "ob.go": "Загрузить треки", "ob.saving": "Сохраняем в", "ob.change": "Изменить", "ob.skip": "Пропустить",
    "ob.welcome.title": "Скачивайте свою музыку", "ob.welcome.sub": "Ищите артистов, вставляйте ссылки на плейлисты и получайте файлы с тегами и обложками в нужном формате.",
    "ob.start": "Начать", "ob.back": "Назад",
    "ob.f1.t": "Ищите", "ob.f1.d": "Артисты, альбомы, песни",
    "ob.f2.t": "Вставляйте", "ob.f2.d": "Плейлисты из любых сервисов",
    "ob.f3.t": "Теги", "ob.f3.d": "Обложки и метаданные",
    "ob.disc": "mpthree-dl — инструмент для личного использования. Вы сами отвечаете за то, что скачиваете, и обязаны соблюдать авторское право и условия используемых сервисов. Мы не храним и не распространяем музыку.",
    "ob.agree": "Я прочитал(а) и принимаю", "ob.and": "и", "ob.continue": "Продолжить",
    "doc.terms": "Условия использования", "doc.privacy": "Политику конфиденциальности", "doc.copyright": "Авторские права и DMCA",

    "s.filter": "Поиск по настройкам", "s.nomatch": "Ничего не найдено", "s.view": "Открыть", "s.legal": "Документы",
    "s.g.general": "Общие", "s.g.advanced": "Дополнительно", "s.g.other": "Прочее",
    "s.p.downloads": "Загрузки", "s.p.system": "Система", "s.p.engine": "Движок", "s.p.tools": "Инструменты", "s.p.about": "О программе",
    "s.files": "Файлы", "s.library": "Библиотека", "s.perf": "Производительность", "s.net": "Сеть и источники",
    "s.dir.hint": "Куда сохраняются готовые треки", "s.org.hint": "Создаёт подпапки Артист / Альбом",
    "s.skip.hint": "Не скачивает один трек дважды", "s.scan.hint": "Считает аудиофайлы в папке загрузок",
    "s.tray.t": "Сворачивать в трей", "s.lang.hint": "Язык интерфейса",
    "s.conc.hint": "Сколько треков качается одновременно", "s.retries.hint": "Попыток, прежде чем трек считается неудачным",
    "s.country.hint": "Двухбуквенный код магазина для поиска по каталогу", "s.cookies.h": "Для контента с возрастным ограничением или входом",
    "s.extra.hint": "Передаются напрямую в yt-dlp", "s.tools.hint": "Помощники, нужные mpthree-dl; ставятся по требованию",
    "s.legal.terms": "Правила использования mpthree-dl", "s.legal.privacy": "Что остаётся на вашем устройстве", "s.legal.copyright": "Уважение к правообладателям",
    "win.min": "Свернуть", "win.max": "Развернуть", "win.restore": "Восстановить", "win.close": "Закрыть",
    "ad.title": "mpthree.fun",
    "ad.text": "Локальный музыкальный плеер, в котором можно всё.",
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

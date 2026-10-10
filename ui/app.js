"use strict";
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const art = (u, n) => (u ? u.replace("100x100", `${n}x${n}`) : "");
const fmtMs = (ms) => fmtSec(ms / 1000);
const fmtSec = (s) => {
  if (!s && s !== 0) return "";
  s = Math.round(s);
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), x = s % 60;
  return h ? `${h}:${String(m).padStart(2, "0")}:${String(x).padStart(2, "0")}` : `${m}:${String(x).padStart(2, "0")}`;
};

const QUALITIES = [
  ["best", "q.best"],
  ["flac", "FLAC"],
  ["wav", "WAV"],
  ["mp3_320", "MP3 320 kbps"],
  ["mp3_v0", "MP3 V0 (~245 kbps)"],
  ["mp3_128", "MP3 128 kbps"],
  ["m4a", "M4A / AAC"],
  ["opus", "Opus"],
];

const S = {
  settings: null,
  tab: "search",
  source: "catalog",
  kind: "artists",
  term: "",
  stack: [],          // page stack inside Search: {type:'artist'|'release', ...}
  songs: [],
  web: [],
  results: null,
  release: null,      // {release, tracks}
  artist: null,       // {artist, releases, off:Set, filters, dedupe}
  link: { groups: [] },
  jobs: new Map(),
  tools: null,
};

// ---------- helpers ----------
let toastTimer;
function toast(msg) {
  const t = $("#toast");
  t.textContent = msg;
  t.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (t.hidden = true), 3800);
}
const errText = (e) => (typeof e === "string" ? e : e?.message || JSON.stringify(e));
const tr = (...a) => t(...a); // `t` is shadowed by local vars in a few functions
const qLabel = (v) => { const q = QUALITIES.find((x) => x[0] === v)?.[1] || ""; return q.startsWith("q.") ? t(q) : q; };
const quality = () => S.settings?.quality || "best";

function movePill() {
  const on = $("#tabs button.on"), pill = $(".nav-pill");
  if (!on || !pill) return;
  pill.style.width = on.offsetWidth + "px";
  pill.style.transform = `translateX(${on.offsetLeft}px)`;
}
window.addEventListener("resize", movePill);
function setTab(tab) {
  S.tab = tab;
  $$("#tabs button").forEach((b) => b.classList.toggle("on", b.dataset.tab === tab));
  movePill();
  for (const v of ["search", "link", "queue", "settings"]) $(`#view-${v}`).hidden = v !== tab;
  if (tab === "settings") refreshTools();
}

async function saveSettings() {
  await invoke("set_settings", { settings: S.settings });
}

// ---------- items ----------
const catItem = (t) => ({ ...t, catalog: true, url: null });
function webItem(e, group, i) {
  const pl = group?.is_playlist;
  if (e.imported) {
    // Metadata from a streaming service: the source is searched at download time.
    return {
      url: null, catalog: true, title: e.title, artist: e.uploader || "", album: e.album || "",
      album_artist: e.album ? e.uploader || "" : "", duration_ms: Math.round((e.duration || 0) * 1000),
      artwork: e.thumbnail || "", subdir: group.title || "Playlist", index: S.link.number ? i + 1 : null,
    };
  }
  return {
    url: e.url,
    title: e.untitled ? "" : e.title,
    artist: e.uploader || "",
    album: pl ? group.title : "",
    subdir: pl ? group.title : null,
    index: pl && S.link.number ? i + 1 : null,
  };
}
async function enqueue(items) {
  if (!items.length) return;
  try {
    const n = await invoke("enqueue", { items, quality: quality() });
    toast(n ? t("toast.added", { n }) : t("toast.already"));
  } catch (e) {
    toast(errText(e));
  }
}

// ---------- preview player ----------
const audio = new Audio();
let playing = null;
function syncPlayButtons() {
  $$(".pv").forEach((b) => b.classList.toggle("on", !!playing && b.dataset.pk === playing.key && !audio.paused));
  $("#p-toggle").classList.toggle("on", !!playing && !audio.paused);
}
async function play(key, getUrl, meta) {
  if (playing?.key === key) {
    audio.paused ? audio.play() : audio.pause();
    return;
  }
  playing = { key, ...meta };
  $("#player").hidden = false;
  $("#p-title").textContent = meta.title;
  $("#p-sub").textContent = meta.sub + " · " + t("preview.loading");
  $("#p-art").src = meta.art || "";
  $("#p-toggle").dataset.pk = key;
  audio.pause();
  syncPlayButtons();
  try {
    const url = await getUrl();
    if (playing?.key !== key) return;
    audio.src = url;
    await audio.play();
    $("#p-sub").textContent = meta.sub;
  } catch (e) {
    if (playing?.key !== key) return;
    toast(t("preview.failed") + errText(e));
    closePlayer();
  }
}
function closePlayer() {
  audio.pause();
  audio.removeAttribute("src");
  playing = null;
  $("#player").hidden = true;
  syncPlayButtons();
}
audio.addEventListener("play", syncPlayButtons);
audio.addEventListener("pause", syncPlayButtons);
audio.addEventListener("ended", () => { $("#p-seek").value = 0; syncPlayButtons(); });
audio.addEventListener("timeupdate", () => {
  if (!audio.duration || !isFinite(audio.duration)) return;
  $("#p-seek").value = (audio.currentTime / audio.duration) * 1000;
  $("#p-cur").textContent = fmtSec(audio.currentTime);
  $("#p-dur").textContent = fmtSec(audio.duration);
});
$("#p-seek").addEventListener("input", (e) => {
  if (audio.duration) audio.currentTime = (e.target.value / 1000) * audio.duration;
});
$("#p-toggle").addEventListener("click", () => (audio.paused ? audio.play() : audio.pause()));
$("#p-close").addEventListener("click", closePlayer);

// ---------- rendering: shared ----------
function trackRow(o) {
  const d = `data-ctx="${o.ctx}" data-i="${o.i}" ${o.g != null ? `data-g="${o.g}"` : ""}`;
  return `<div class="row">
    ${o.check !== undefined ? `<input type="checkbox" data-act="chk" ${d} ${o.check ? "checked" : ""}>` : ""}
    ${o.art ? `<img class="thumb" loading="lazy" src="${esc(o.art)}" alt="">` : ""}
    ${o.num != null ? `<span class="num">${o.num}</span>` : ""}
    <button class="icon pv ${playing?.key === o.pk && !audio.paused ? "on" : ""}" data-act="preview" data-pk="${esc(o.pk)}" ${d} title="${t("tip.preview")}"></button>
    <div class="grow"><div class="t">${esc(o.title)}${o.explicit ? '<span class="tag">E</span>' : ""}</div><div class="s">${esc(o.sub)}</div></div>
    <span class="dur">${o.dur || ""}</span>
    <button class="icon" data-act="dl" ${d} title="${t("tip.download")}">${ICON.dl}</button>
  </div>`;
}
const catRow = (t, ctx, i, num) =>
  trackRow({
    ctx, i, num,
    pk: "c:" + t.id,
    art: ctx === "songs" ? art(t.artwork, 100) : null,
    title: t.title,
    explicit: t.explicit,
    sub: ctx === "songs" ? `${t.artist} — ${t.album}` : t.artist,
    dur: fmtMs(t.duration_ms),
  });
function webRow(e, ctx, i, extra = {}) {
  return trackRow({
    ctx, i, ...extra,
    pk: "w:" + (e.url || e.uploader + "|" + e.title),
    art: e.thumbnail,
    title: e.title,
    sub: e.uploader,
    dur: fmtSec(e.duration),
  });
}
const I = (d) => `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">${d}</svg>`;
const ICON = {
  dl: I('<path d="M12 4v11m0 0l-5-5m5 5l5-5M5 20h14"/>'),
  x: I('<path d="M6 6l12 12M18 6L6 18"/>'),
  retry: I('<path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7"/>'),
  folder: I('<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>'),
};
const initials = (n) => (n || "?").trim().slice(0, 1).toUpperCase();

// ---------- search ----------
const isCatalog = () => S.source === "catalog" || S.source === "deezer";
const catalogProvider = () => (S.source === "deezer" ? "deezer" : null);

function renderSearch() {
  const top = S.stack[S.stack.length - 1];
  $("#search-bar").hidden = !!top;
  const body = $("#search-body");
  if (top?.type === "artist") return renderArtist(body);
  if (top?.type === "release") return renderRelease(body);

  $("#seg-kind").hidden = !isCatalog();
  if (S.results === "loading") return (body.innerHTML = `<div class="empty">${t("search.searching")}</div>`);
  if (S.results === null) {
    body.innerHTML = `<div class="hero">
      <div class="hero-ic">${ICON.dl.replace('width="14" height="14"', 'width="26" height="26"')}</div>
      <h2>${t("hero.title")}</h2>
      <p class="muted">${t("hero.text")}</p>
    </div>`;
    return;
  }
  if (!S.results.length) return (body.innerHTML = `<div class="empty">${t("search.none")}</div>`);

  if (!isCatalog()) {
    S.web = S.results;
    body.innerHTML = S.web.map((e, i) => webRow(e, "web", i)).join("");
  } else if (S.kind === "artists") {
    body.innerHTML = S.results
      .map((a) => `<div class="row click" data-act="open-artist" data-id="${a.id}">
        <div class="avatar">${esc(initials(a.name))}</div>
        <div class="grow"><div class="t">${esc(a.name)}</div><div class="s">${esc(a.genre)}</div></div><span class="muted">›</span></div>`)
      .join("");
  } else if (S.kind === "releases") {
    body.innerHTML = `<div class="grid">${S.results.map(releaseCard).join("")}</div>`;
  } else {
    S.songs = S.results;
    body.innerHTML = S.songs.map((t, i) => catRow(t, "songs", i)).join("");
  }
}

const kindLabel = (k) => t(k === "ep" ? "kind.ep" : k === "single" ? "kind.single" : "kind.album");
function releaseCard(r, opts = {}) {
  const kind = kindLabel(r.kind);
  return `<div class="card ${opts.off ? "off" : ""}" data-act="open-release" data-id="${r.id}">
    <img loading="lazy" src="${esc(art(r.artwork, 300))}" alt="">
    ${opts.pick ? `<label class="pick" data-stop="1"><input type="checkbox" data-act="pick" data-id="${r.id}" ${opts.off ? "" : "checked"}></label>` : ""}
    <div class="t">${esc(r.name)}${r.explicit ? '<span class="tag">E</span>' : ""}</div>
    <div class="s">${opts.pick ? "" : esc(r.artist) + " · "}${esc(r.year)} · ${kind}</div>
  </div>`;
}

let searchSeq = 0;
async function doSearch() {
  const term = $("#q").value.trim();
  if (!term) return;
  const seq = ++searchSeq;
  S.term = term;
  S.stack = [];
  S.results = "loading";
  renderSearch();
  let res;
  try {
    res =
      isCatalog()
        ? await invoke("search_catalog", { kind: S.kind, term, provider: catalogProvider() })
        : await invoke("search_web", { provider: S.source, term });
  } catch (e) {
    res = [];
    if (seq === searchSeq) toast(errText(e));
  }
  if (seq !== searchSeq) return; // a newer search superseded this one
  S.results = res;
  renderSearch();
}

async function openArtist(id) {
  const body = $("#search-body");
  S.stack.push({ type: "artist", loading: true });
  $("#search-bar").hidden = true;
  body.innerHTML = `<div class="empty">${t("artist.loading")}</div>`;
  try {
    const r = await invoke("artist_releases", { artistId: id, provider: catalogProvider() });
    S.artist = { artist: r.artist, releases: r.releases, off: new Set(), filters: { album: true, ep: true, single: true }, dedupe: true, primary: true };
    S.stack[S.stack.length - 1] = { type: "artist" };
  } catch (e) {
    S.stack.pop();
    toast(errText(e));
  }
  renderSearch();
}

const editionKey = (r) =>
  r.name
    .toLowerCase()
    .replace(/[(\[][^)\]]*\b(deluxe|expanded|remaster(?:ed)?|edition|bonus|anniversary|explicit|clean|special|collector'?s|super)\b[^)\]]*[)\]]/g, "")
    .replace(/\s[-–]\s.*\b(deluxe|edition|remaster(?:ed)?)\b.*$/, "")
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim() + "|" + r.kind;

function visibleReleases() {
  const a = S.artist;
  const name = (a.artist?.name || "").toLowerCase();
  let list = a.releases.filter((r) => a.filters[r.kind] && (!a.primary || r.artist.toLowerCase() === name));
  if (a.dedupe) {
    // Keep one edition per release: the original (earliest), then the fullest, then the explicit one.
    const better = (r, cur) =>
      r.year !== cur.year ? r.year < cur.year
      : r.track_count !== cur.track_count ? r.track_count > cur.track_count
      : r.explicit && !cur.explicit;
    const best = new Map();
    for (const r of list) {
      const k = editionKey(r), cur = best.get(k);
      if (!cur || better(r, cur)) best.set(k, r);
    }
    const keep = new Set([...best.values()].map((r) => r.id));
    list = list.filter((r) => keep.has(r.id));
  }
  return list;
}
const selectedReleases = () => visibleReleases().filter((r) => !S.artist.off.has(r.id));

function renderArtist(body) {
  const a = S.artist, name = a.artist?.name || a.releases[0]?.artist || t("artist");
  const count = (k) => a.releases.filter((r) => r.kind === k).length;
  const vis = visibleReleases(), sel = selectedReleases();
  const tracks = sel.reduce((n, r) => n + r.track_count, 0);
  body.innerHTML = `
    <button class="btn back" data-act="back">${t("back")}</button>
    <div class="head">
      <div class="avatar big">${esc(initials(name))}</div>
      <div>
        <div class="muted">${esc(a.artist?.genre || t("artist"))}</div>
        <h1>${esc(name)}</h1>
        <div class="muted">${t("artist.count", { n: a.releases.length })}</div>
        <div class="acts">
          <button class="btn primary" data-act="dl-discog" ${sel.length ? "" : "disabled"}>${t("artist.dl", { n: sel.length })}${tracks ? t("artist.dl.tracks", { t: tracks }) : ""}</button>
          <button class="btn" data-act="sel-all">${t("sel.all")}</button>
          <button class="btn" data-act="sel-none">${t("sel.none")}</button>
        </div>
      </div>
    </div>
    <div class="filters">
      <label><input type="checkbox" data-act="filter" data-k="album" ${a.filters.album ? "checked" : ""}> ${t("f.albums")} ${count("album")}</label>
      <label><input type="checkbox" data-act="filter" data-k="ep" ${a.filters.ep ? "checked" : ""}> ${t("f.eps")} ${count("ep")}</label>
      <label><input type="checkbox" data-act="filter" data-k="single" ${a.filters.single ? "checked" : ""}> ${t("f.singles")} ${count("single")}</label>
      <label><input type="checkbox" data-act="primary" ${a.primary ? "checked" : ""}> ${t("f.primary", { name: esc(name) })}</label>
      <label><input type="checkbox" data-act="dedupe" ${a.dedupe ? "checked" : ""}> ${t("f.dedupe")}</label>
    </div>
    ${vis.length ? `<div class="grid">${vis.map((r) => releaseCard(r, { pick: true, off: a.off.has(r.id) })).join("")}</div>` : `<div class="empty">${t("no.match")}</div>`}`;
}

async function openRelease(id) {
  const body = $("#search-body");
  S.stack.push({ type: "release", loading: true });
  $("#search-bar").hidden = true;
  body.innerHTML = `<div class="empty">${t("release.loading")}</div>`;
  try {
    S.release = await invoke("release_tracks", { releaseId: id, provider: catalogProvider() });
    S.stack[S.stack.length - 1] = { type: "release" };
  } catch (e) {
    S.stack.pop();
    toast(errText(e));
  }
  renderSearch();
}

function renderRelease(body) {
  const { release: r, tracks } = S.release;
  const kind = kindLabel(r.kind);
  const mins = Math.round(tracks.reduce((n, t) => n + t.duration_ms, 0) / 60000);
  body.innerHTML = `
    <button class="btn back" data-act="back">← Back</button>
    <div class="head">
      <img src="${esc(art(r.artwork, 400))}" alt="">
      <div>
        <div class="muted">${kind}</div>
        <h1>${esc(r.name)}</h1>
        <div><a class="link" data-act="open-artist" data-id="${r.artist_id}" style="cursor:pointer;text-decoration:underline">${esc(r.artist)}</a> · ${esc(r.year)} · ${t("release.meta", { n: tracks.length, m: mins })}${r.genre ? " · " + esc(r.genre) : ""}</div>
        <div class="acts">
          <button class="btn primary" data-act="dl-release">${t("release.dl", { kind: kind.toLowerCase() })}</button>
          <span class="muted">${t("release.previews")}</span>
        </div>
      </div>
    </div>
    ${tracks.map((t, i) => catRow(t, "release", i, t.disc_total > 1 ? `${t.disc_no}.${t.track_no}` : t.track_no)).join("")}`;
}

// ---------- link ----------
// "Artist - Title" lines -> an importable group (matched against the web at download time).
function parseTrackList(text) {
  const entries = text
    .split(/\r?\n/)
    .map((l) => l.replace(/^\s*(?:\d+[.)]\s*)?/, "").trim())
    .filter(Boolean)
    .map((l, i) => {
      const m = l.match(/^(.+?)\s+[-–—]\s+(.+)$/);
      return { id: String(i), title: m ? m[2].trim() : l, uploader: m ? m[1].trim() : "", album: "", url: "", duration: null, thumbnail: "", imported: true };
    });
  return { title: t("link.tracklist"), uploader: "", is_playlist: true, extractor: t("link.pasted"), entries, note: "" };
}

async function fetchLinks() {
  const text = $("#link-in").value;
  const urls = text.split(/\s+/).filter((u) => /^https?:\/\//i.test(u));
  const body = $("#link-body"), btn = $("#link-go");
  S.link.groups = [];
  if (!urls.length) {
    if (!text.trim()) return toast(t("link.paste"));
    const g = parseTrackList(text);
    g.sel = new Set(g.entries.map((_, i) => i));
    S.link.groups.push(g);
    return renderLink();
  }
  btn.disabled = true;
  const errors = [];
  for (let n = 0; n < urls.length; n++) {
    body.innerHTML = `<div class="empty">${t("link.reading", { i: n + 1, n: urls.length })}</div>`;
    try {
      // Streaming services are read as metadata; everything else goes through yt-dlp.
      const g = (await invoke("import_playlist", { url: urls[n] })) || (await invoke("probe_url", { url: urls[n] }));
      g.sel = new Set(g.entries.map((_, i) => i));
      S.link.groups.push(g);
    } catch (e) {
      errors.push(`${urls[n]}
${errText(e)}`);
    }
  }
  btn.disabled = false;
  renderLink(errors);
}

function renderLink(errors = []) {
  const body = $("#link-body");
  const gs = S.link.groups;
  if (!gs.length && !errors.length) return (body.innerHTML = "");
  const total = gs.reduce((n, g) => n + g.sel.size, 0);
  body.innerHTML =
    errors.map((e) => `<div class="empty" style="padding:14px 0;color:var(--bad);white-space:pre-wrap">${esc(e)}</div>`).join("") +
    gs
      .map((g, gi) => `
      <div class="group-head">
        <input type="checkbox" data-act="g-all" data-g="${gi}" ${g.sel.size === g.entries.length ? "checked" : ""}>
        <b>${esc(g.title || g.entries[0].title)}</b>
        <span class="muted">${g.is_playlist ? t("link.tracks", { n: g.entries.length }) : t("link.single")} · ${esc(g.extractor)}${g.uploader ? " · " + esc(g.uploader) : ""}</span>
      </div>
      ${g.note ? `<div class="muted" style="margin:0 0 6px 27px">${esc(g.note)}</div>` : ""}
      ${g.entries[0]?.imported ? `<div class="muted" style="margin:0 0 6px 27px">${t("link.meta")}</div>` : ""}
      ${g.entries.map((e, i) => webRow(e, "link", i, { g: gi, check: g.sel.has(i), num: g.is_playlist ? i + 1 : null })).join("")}`)
      .join("") +
    (gs.length
      ? `<div id="link-foot"><button class="btn primary" data-act="link-dl" ${total ? "" : "disabled"}>${t("link.dl", { n: total })}</button><span class="muted">${t("link.as", { q: esc(qLabel(quality())) })}</span></div>`
      : "");
}

// ---------- queue ----------
const ACTIVE = new Set(["queued", "resolving", "downloading", "converting", "tagging"]);

function jobHtml(v) {
  const detail = v.state === "error" ? v.error : v.note || [v.artist, v.album].filter(Boolean).join(" · ");
  const status = v.state === "downloading" && v.speed ? `${Math.round(v.progress * 100)}% · ${v.speed}${v.eta ? " · " + v.eta : ""}` : t("st." + v.state);
  let act = "";
  if (ACTIVE.has(v.state)) act = `<button class="icon" data-act="job-cancel" data-id="${v.id}" title="${t("tip.cancel")}">${ICON.x}</button>`;
  else if (v.state === "error" || v.state === "cancelled") act = `<button class="icon" data-act="job-retry" data-id="${v.id}" title="${t("tip.retry")}">${ICON.retry}</button>`;
  else if (v.state === "done") act = `<button class="icon" data-act="job-reveal" data-id="${v.id}" title="${t("tip.reveal")}">${ICON.folder}</button>`;
  return `<div class="job ${v.state}" id="job-${v.id}">
    <div class="top">
      <div class="grow"><div class="t">${esc(v.title)}</div><div class="s" title="${esc(detail)}">${esc(detail)}</div></div>
      <div class="st">${esc(status)}</div>${act}
    </div>
    <div class="meter"><i style="width:${Math.round(v.progress * 100)}%"></i></div>
  </div>`;
}

function upsertJob(v) {
  S.jobs.set(v.id, v);
  const el = $(`#job-${v.id}`);
  if (el) el.outerHTML = jobHtml(v);
  else $("#jobs").insertAdjacentHTML("beforeend", jobHtml(v));
  updateQueueMeta();
}
function updateQueueMeta() {
  const all = [...S.jobs.values()];
  const active = all.filter((j) => ACTIVE.has(j.state)).length;
  const done = all.filter((j) => j.state === "done").length;
  const failed = all.filter((j) => j.state === "error").length;
  const b = $("#badge");
  b.hidden = !active;
  b.textContent = active;
  $("#jobs-empty").hidden = all.length > 0;
  $("#queue-summary").textContent = all.length ? t("queue.summary", { a: active, d: done }) + (failed ? t("queue.failed", { f: failed }) : "") : "";
}
function rebuildJobs() {
  $("#jobs").innerHTML = [...S.jobs.values()].map(jobHtml).join("");
  updateQueueMeta();
}

// ---------- window controls (the native frame is off) ----------
const WIN_ICON = {
  min: '<path d="M5 12h14"/>',
  max: '<rect x="5" y="5" width="14" height="14" rx="2"/>',
  restore: '<rect x="8" y="5" width="11" height="11" rx="2"/><path d="M5 9v8a2 2 0 0 0 2 2h8"/>',
  close: '<path d="M6 6l12 12M18 6L6 18"/>',
};
const winSvg = (k) => `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${WIN_ICON[k]}</svg>`;
const appWin = () => window.__TAURI__.window.getCurrentWindow();
let winMax = false;
function renderWinControls() {
  const html = `<button data-w="min" title="${t("win.min")}" aria-label="${t("win.min")}">${winSvg("min")}</button>
    <button data-w="max" title="${t(winMax ? "win.restore" : "win.max")}" aria-label="${t("win.max")}">${winSvg(winMax ? "restore" : "max")}</button>
    <button data-w="close" class="x" title="${t("win.close")}" aria-label="${t("win.close")}">${winSvg("close")}</button>`;
  $$(".wc").forEach((el) => (el.innerHTML = html));
}
document.addEventListener("click", (e) => {
  const b = e.target.closest(".wc button"); if (!b) return;
  const w = appWin();
  if (b.dataset.w === "min") w.minimize();
  else if (b.dataset.w === "max") w.toggleMaximize();
  else w.close();
});
async function syncWinState() {
  try { const m = await appWin().isMaximized(); if (m !== winMax) { winMax = m; document.documentElement.classList.toggle("maxed", m); renderWinControls(); } } catch {}
}
try { appWin().onResized(syncWinState); } catch {}

// ---------- settings navigation: branched menu (tree with drawn branches) ----------
const BM = { row: 34, indent: 40, trunk: 14, radius: 10, pad: 6, mark: 16 };
const mkIcon = (p) => `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${p}</svg>`;
const ST_GROUPS = [
  { id: "general", key: "s.g.general", items: [
    { id: "downloads", key: "s.p.downloads", icon: mkIcon('<path d="M12 3v12M7 10l5 5 5-5M4 21h16"/>') },
    { id: "system", key: "s.p.system", icon: mkIcon('<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1"/>') },
  ] },
  { id: "advanced", key: "s.g.advanced", items: [
    { id: "engine", key: "s.p.engine", icon: mkIcon('<path d="M13 2L4 14h7l-1 8 9-12h-7z"/>') },
    { id: "tools", key: "s.p.tools", icon: mkIcon('<path d="M14.7 6.3a4 4 0 0 0-5.4 5.4L3 18l3 3 6.3-6.3a4 4 0 0 0 5.4-5.4l-2.4 2.4-2.6-.6-.6-2.6z"/>') },
  ] },
  { id: "other", key: "s.g.other", items: [
    { id: "about", key: "s.p.about", icon: mkIcon('<circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/>') },
  ] },
];
const stOpen = new Set(ST_GROUPS.map((g) => g.id));
let stPane = "downloads";
const stItem = (id) => ST_GROUPS.flatMap((g) => g.items).find((i) => i.id === id);
const bmRowY = (k) => BM.pad + k * BM.row + BM.row / 2;
const bmR = Math.min(BM.radius, BM.row / 2 - 2), bmEnd = BM.indent - 8;
const bmBranch = (k) => `M ${BM.trunk} ${bmRowY(k) - bmR} A ${bmR} ${bmR} 0 0 0 ${BM.trunk + bmR} ${bmRowY(k)} H ${bmEnd}`;
const bmReach = (k) => `M ${BM.trunk} 0 V ${bmRowY(k) - bmR} A ${bmR} ${bmR} 0 0 0 ${BM.trunk + bmR} ${bmRowY(k)} H ${bmEnd}`;
const bmLen = (k) => bmRowY(k) - bmR + (Math.PI * bmR) / 2 + (bmEnd - BM.trunk - bmR);

function renderMenu() {
  const q = $("#st-q").value.trim().toLowerCase();
  const groups = ST_GROUPS.map((g) => ({ ...g, items: g.items.filter((i) => !q || t(i.key).toLowerCase().includes(q)) })).filter((g) => g.items.length);
  $("#st-none").hidden = groups.length > 0;
  $("#st-menu").innerHTML = `<div class="bm"><span class="bm-marker" aria-hidden="true"></span>${groups.map((g) => {
    const h = BM.pad * 2 + g.items.length * BM.row;
    const open = q || stOpen.has(g.id);
    return `<div class="bm-section" data-g="${g.id}" ${open ? "data-open" : ""}>
      <button class="bm-head" aria-expanded="${!!open}" data-g="${g.id}">${esc(t(g.key))}</button>
      <div class="bm-body"><div class="bm-fold"><div class="bm-tree" style="height:${h}px">
        <svg class="bm-lines" width="${BM.indent}" height="${h}" aria-hidden="true">
          <path class="bm-base" d="M ${BM.trunk} 0 V ${bmRowY(g.items.length - 1) - bmR} ${g.items.map((_, k) => bmBranch(k)).join(" ")}"/>
          ${g.items.map((i, k) => `<path class="bm-reach" d="${bmReach(k)}" data-len="${bmLen(k)}" data-id="${i.id}" style="stroke-dasharray:${bmLen(k)};stroke-dashoffset:${i.id === stPane ? 0 : bmLen(k)}"/>`).join("")}
        </svg>
        ${g.items.map((i) => `<button class="bm-item" data-pane="${i.id}" ${i.id === stPane ? "data-active" : ""} tabindex="${open ? 0 : -1}"><span class="bm-icon">${i.icon}</span><span class="bm-label">${esc(t(i.key))}</span></button>`).join("")}
      </div></div></div></div>`;
  }).join("")}</div>`;
  placeMarker(false);
}
function placeMarker(glide) {
  const m = $(".bm-marker"); if (!m) return;
  const g = ST_GROUPS.find((x) => x.items.some((i) => i.id === stPane));
  const sec = g && $(`.bm-section[data-g="${g.id}"]`);
  const head = sec?.querySelector(".bm-head");
  const on = sec && sec.hasAttribute("data-open") && head;
  if (!glide) m.style.transition = "none";
  if (on) m.style.top = head.offsetTop + (head.offsetHeight - BM.mark) / 2 + "px";
  m.toggleAttribute("data-on", !!on);
  if (!glide) { void m.offsetHeight; m.style.transition = ""; }
}
function setPane(id) {
  stPane = id;
  $$(".bm-item").forEach((b) => b.toggleAttribute("data-active", b.dataset.pane === id));
  $$(".bm-reach").forEach((p) => (p.style.strokeDashoffset = p.dataset.id === id ? 0 : p.dataset.len));
  placeMarker(true);
  $$(".st-pane").forEach((p) => {
    const on = p.dataset.pane === id;
    p.hidden = !on;
    if (on) { p.classList.remove("enter"); void p.offsetWidth; p.classList.add("enter"); }
  });
  const it = stItem(id);
  $("#st-title").innerHTML = it ? it.icon + `<span>${esc(t(it.key))}</span>` : "";
}
const filterSettings = renderMenu;
$("#st-q").addEventListener("input", renderMenu);
$("#st-menu").addEventListener("click", (e) => {
  const head = e.target.closest(".bm-head");
  if (head) {
    const sec = head.parentElement, g = head.dataset.g;
    const open = !sec.hasAttribute("data-open");
    sec.toggleAttribute("data-open", open);
    head.setAttribute("aria-expanded", String(open));
    sec.querySelectorAll(".bm-item").forEach((b) => (b.tabIndex = open ? 0 : -1));
    open ? stOpen.add(g) : stOpen.delete(g);
    return placeMarker(true);
  }
  const b = e.target.closest(".bm-item"); if (b) setPane(b.dataset.pane);
});
$("#st-q").addEventListener("input", filterSettings);
function fillLangSelect() {
  const sel = $("#s-lang");
  sel.innerHTML = LANGS.map(([c, n]) => `<option value="${c}">${esc(n)}</option>`).join("");
  sel.value = LANG;
}
$("#s-lang").addEventListener("change", (e) => setLang(e.target.value));

// ---------- settings & tools ----------
function loadSettingsUi() {
  const s = S.settings;
  $("#s-dir").value = s.output_dir;
  $("#s-org").checked = s.organize;
  $("#s-skip").checked = s.skip_existing;
  $("#s-tray").checked = s.close_to_tray;
  $("#s-conc").value = s.concurrency;
  $("#s-country").value = s.country;
  $("#s-cookies").value = s.cookies_browser;
  $("#s-extra").value = s.extra_args;
  $("#s-retries").value = s.retries;
}

async function refreshTools() {
  try {
    S.tools = await invoke("tools_status");
  } catch (e) {
    return;
  }
  const t = S.tools;
  $("#t-ytdlp").textContent = t.ytdlp ? `${t.ytdlp.version} — ${t.ytdlp.path}` : tr("s.notfound");
  $("#t-ytdlp-btn").textContent = t.ytdlp ? tr("s.update") : tr("s.install");
  $("#t-ffmpeg").textContent = t.ffmpeg ? `${t.ffmpeg.version} — ${t.ffmpeg.path}` : tr("s.ffmpeg.missing");
  $("#t-ffmpeg-btn").textContent = tr("s.install");
  $("#t-ffmpeg-btn").hidden = !!t.ffmpeg || t.platform !== "windows";
  const missing = [!t.ytdlp && "yt-dlp", !t.ffmpeg && "ffmpeg"].filter(Boolean);
  const bn = $("#banner");
  bn.hidden = !missing.length;
  if (missing.length) bn.innerHTML = `${tr("banner.text", { tools: missing.join(", ") })} <a data-act="goto-settings">${tr("banner.link")}</a> ${tr("banner.tail")}`;
}

async function toolAction(btn, cmd, tool) {
  const label = btn.textContent;
  btn.disabled = true;
  btn.textContent = t("s.working");
  try {
    const r = await invoke(cmd);
    if (typeof r === "string" && r) toast(r.split("\n").pop());
  } catch (e) {
    toast(errText(e));
  }
  btn.disabled = false;
  btn.textContent = label;
  refreshTools();
}

// ---------- events ----------
document.addEventListener("click", async (ev) => {
  const el = ev.target.closest("[data-act]");
  if (ev.target.closest("[data-stop]")) return;
  if (!el) return;
  const { act, ctx, i, g, id } = el.dataset;
  switch (act) {
    case "chk": case "pick": case "filter": case "dedupe": case "primary": case "g-all": return; // handled on change
    case "goto-settings": return setTab("settings");
    case "open-site": return openSite();
    case "back":
      S.stack.pop();
      return renderSearch();
    case "open-artist":
      return openArtist(Number(id));
    case "open-release":
      return openRelease(Number(id));
    case "sel-all": S.artist.off.clear(); return renderSearch();
    case "sel-none": visibleReleases().forEach((r) => S.artist.off.add(r.id)); return renderSearch();
    case "dl-discog": {
      const ids = selectedReleases().map((r) => r.id);
      toast(t("reading.releases", { n: ids.length }));
      return invoke("enqueue_releases", { ids, quality: quality(), provider: catalogProvider() }).catch((e) => toast(errText(e)));
    }
    case "dl-release":
      return enqueue(S.release.tracks.map(catItem));
    case "preview": case "dl": {
      const row = rowData(el);
      if (!row) return;
      if (act === "dl") return enqueue([row.item]);
      return play(el.dataset.pk, row.preview, row.meta);
    }
    case "link-dl": {
      const items = [];
      S.link.groups.forEach((gr) => gr.sel.forEach((idx) => items.push(webItem(gr.entries[idx], gr, idx))));
      return enqueue(items);
    }
    case "job-cancel": return invoke("cancel_job", { id: Number(id) });
    case "job-retry": return invoke("retry_job", { id: Number(id) });
    case "job-reveal": return invoke("open_path", { path: S.jobs.get(Number(id)).path }).catch((e) => toast(errText(e)));
  }
});

// Describe the row behind a preview/download button.
function rowData(el) {
  const { ctx, i, g } = el.dataset;
  if (ctx === "songs" || ctx === "release") {
    const t = (ctx === "songs" ? S.songs : S.release.tracks)[i];
    return {
      item: catItem(t),
      preview: async () => { if (!t.preview_url) throw "No preview available"; return t.preview_url; },
      meta: { title: t.title, sub: `${t.artist} · ${tr("preview.30")}`, art: art(t.artwork, 100) },
    };
  }
  const grp = ctx === "link" ? S.link.groups[g] : null;
  const e = ctx === "link" ? grp.entries[i] : S.web[i];
  return {
    item: webItem(e, grp, Number(i)),
    preview: () => invoke("stream_url", { url: e.url || `ytsearch1:${e.uploader} ${e.title}` }),
    meta: { title: e.title, sub: e.uploader, art: e.thumbnail },
  };
}

document.addEventListener("change", (ev) => {
  const el = ev.target.closest("[data-act]");
  if (!el) return;
  const { act, id, g, i, k } = el.dataset;
  if (act === "pick") {
    el.checked ? S.artist.off.delete(Number(id)) : S.artist.off.add(Number(id));
    return renderSearch();
  }
  if (act === "filter") { S.artist.filters[k] = el.checked; return renderSearch(); }
  if (act === "dedupe") { S.artist.dedupe = el.checked; return renderSearch(); }
  if (act === "primary") { S.artist.primary = el.checked; return renderSearch(); }
  if (act === "chk") {
    const sel = S.link.groups[g].sel;
    el.checked ? sel.add(Number(i)) : sel.delete(Number(i));
    return renderLink();
  }
  if (act === "g-all") {
    const gr = S.link.groups[g];
    gr.sel = new Set(el.checked ? gr.entries.map((_, n) => n) : []);
    return renderLink();
  }
});

$("#tabs").addEventListener("click", (e) => { const b = e.target.closest("button"); if (b) setTab(b.dataset.tab); });
$("#q-go").addEventListener("click", doSearch);
$("#q").addEventListener("keydown", (e) => e.key === "Enter" && doSearch());
$("#seg-source").addEventListener("click", (e) => {
  const b = e.target.closest("button"); if (!b) return;
  S.source = b.dataset.v;
  $$("#seg-source button").forEach((x) => x.classList.toggle("on", x === b));
  S.results = null;
  S.term && $("#q").value.trim() ? doSearch() : renderSearch();
});
$("#seg-kind").addEventListener("click", (e) => {
  const b = e.target.closest("button"); if (!b) return;
  S.kind = b.dataset.v;
  $$("#seg-kind button").forEach((x) => x.classList.toggle("on", x === b));
  S.results = null;
  $("#q").value.trim() ? doSearch() : renderSearch();
});
$("#link-go").addEventListener("click", fetchLinks);
$("#link-in").addEventListener("keydown", (e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); fetchLinks(); } });
$("#link-num").addEventListener("change", (e) => (S.link.number = e.target.checked));

$("#s-skip").addEventListener("change", (e) => { S.settings.skip_existing = e.target.checked; saveSettings(); });
$("#s-tray").addEventListener("change", (e) => { S.settings.close_to_tray = e.target.checked; saveSettings(); });
$("#s-scan").addEventListener("click", async () => {
  const out = $("#s-scan-res");
  out.textContent = t("s.scanning");
  try { out.textContent = t("s.found", { n: await invoke("scan_library") }); } catch (e) { out.textContent = String(e); }
});
$("#s-org").addEventListener("change", (e) => { S.settings.organize = e.target.checked; saveSettings(); });
$("#s-conc").addEventListener("change", (e) => { S.settings.concurrency = Math.min(8, Math.max(1, Number(e.target.value) || 3)); e.target.value = S.settings.concurrency; saveSettings(); });
$("#s-country").addEventListener("change", (e) => { S.settings.country = (e.target.value.trim() || "US").toUpperCase(); e.target.value = S.settings.country; saveSettings(); });
$("#s-cookies").addEventListener("change", (e) => { S.settings.cookies_browser = e.target.value; saveSettings(); });
$("#s-retries").addEventListener("change", (e) => { S.settings.retries = Math.min(10, Math.max(0, Number(e.target.value) || 0)); e.target.value = S.settings.retries; saveSettings(); });
$("#s-extra").addEventListener("change", (e) => { S.settings.extra_args = e.target.value; saveSettings(); });
$("#s-browse").addEventListener("click", async () => {
  const dir = await window.__TAURI__.dialog.open({ directory: true, defaultPath: S.settings.output_dir });
  if (dir) { S.settings.output_dir = dir; $("#s-dir").value = dir; saveSettings(); }
});
$("#s-open").addEventListener("click", () => invoke("open_path", { path: S.settings.output_dir }).catch((e) => toast(errText(e))));
$("#q-folder").addEventListener("click", () => $("#s-open").click());
$("#q-cancel").addEventListener("click", () => invoke("cancel_all"));
$("#q-clear").addEventListener("click", async () => {
  await invoke("clear_finished");
  for (const [id, j] of [...S.jobs]) if (!ACTIVE.has(j.state)) S.jobs.delete(id);
  rebuildJobs();
});
$("#t-ytdlp-btn").addEventListener("click", (e) => toolAction(e.target, S.tools?.ytdlp ? "update_ytdlp" : "install_ytdlp"));
$("#t-ffmpeg-btn").addEventListener("click", (e) => toolAction(e.target, "install_ffmpeg"));

listen("job-update", (e) => upsertJob(e.payload));
listen("notice", (e) => toast(e.payload));
listen("tool-progress", (e) => {
  const btn = $(e.payload.tool === "ffmpeg" ? "#t-ffmpeg-btn" : "#t-ytdlp-btn");
  btn.textContent = t("s.downloading", { pct: e.payload.pct });
});

// ---------- language ----------
const qShort = (v) => (v === "best" ? t("q.short.best") : QUALITIES.find((q) => q[0] === v)?.[1] || v);
const SWITCHERS = {
  lang: {
    options: () => LANGS.map(([value, label]) => ({ value, label })),
    value: () => LANG,
    pick: (v) => setLang(v),
  },
  quality: {
    options: () => QUALITIES.map(([value, l]) => ({ value, label: l.startsWith("q.") ? t(l) : l })),
    value: quality,
    pick: (v) => { S.settings.quality = v; saveSettings(); syncSwitchers(); renderLink(); },
  },
};
function syncSwitchers() {
  $$('[data-sw="quality"] .sw-val').forEach((el) => (el.textContent = qShort(quality())));
}
// Icon button that opens a small radio menu, like the language / theme switchers in mpthree-desktop onboarding.
let openMenu = null;
function closeMenu() { openMenu?.remove(); openMenu = null; $$(".sw[aria-expanded=true]").forEach((b) => b.setAttribute("aria-expanded", "false")); }
function toggleMenu(btn) {
  const was = btn.getAttribute("aria-expanded") === "true";
  closeMenu();
  if (was) return;
  const cfg = SWITCHERS[btn.dataset.sw], cur = cfg.value();
  const m = document.createElement("div");
  m.className = "popover"; m.setAttribute("role", "menu");
  m.innerHTML = cfg.options().map((o) => `<button role="menuitemradio" aria-checked="${o.value === cur}" data-v="${esc(o.value)}" class="${o.value === cur ? "sel" : ""}">${esc(o.label)}${o.value === cur ? CHECK : ""}</button>`).join("");
  document.body.appendChild(m);
  const r = btn.getBoundingClientRect();
  m.style.top = r.bottom + 6 + "px";
  m.style.right = Math.max(8, innerWidth - r.right) + "px";
  btn.setAttribute("aria-expanded", "true");
  m.addEventListener("click", (e) => { const b = e.target.closest("button"); if (!b) return; cfg.pick(b.dataset.v); closeMenu(); });
  openMenu = m;
}
const CHECK = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>';
document.addEventListener("pointerdown", (e) => {
  if (!openMenu) return;
  if (e.target.closest(".popover")) return;
  if (e.target.closest(".sw")) return; // the button toggles itself
  closeMenu();
});
document.addEventListener("click", (e) => { const b = e.target.closest(".sw"); if (b) toggleMenu(b); });
document.addEventListener("keydown", (e) => { if (e.key === "Escape") closeMenu(); });
function renderAll() {
  applyI18n();
  fillLangSelect();
  setPane(stPane);
  filterSettings();
  renderWinControls();
  movePill();
  syncSwitchers();
  obRender();
  if (S.results !== "loading") renderSearch();
  renderLink();
  rebuildJobs();
  refreshTools();
}
function setLang(code) {
  if (!DICT[code]) return;
  LANG = code;
  S.settings.language = code;
  saveSettings();
  renderAll();
}

// ---------- ad banner ----------
const SITE = "https://mpthree.fun";
function openSite() {
  window.__TAURI__.opener.openUrl(SITE).catch((e) => toast(errText(e)));
}
document.addEventListener("keydown", (e) => {
  const a = e.target.closest?.('[data-act="open-site"]');
  if (a && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); openSite(); }
});

// ---------- onboarding ----------
function finishOnboarding() {
  $("#onboard").hidden = true;
  S.settings.onboarded = true;
  saveSettings();
}
let obStep = 0, obMode = "artist", obConsentOnly = false;
const DOC_BASE = "https://mpthreedl.vercel.app/";
function openDoc(name) { window.__TAURI__.opener.openUrl(DOC_BASE + name).catch((e) => toast(errText(e))); }
document.addEventListener("click", (e) => { const a = e.target.closest("[data-doc]"); if (a) { e.preventDefault(); openDoc(a.dataset.doc); } });
document.addEventListener("keydown", (e) => { const a = e.target.closest?.("[data-doc]"); if (a && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); openDoc(a.dataset.doc); } });
function obRender() {
  $$(".ob-step").forEach((el) => (el.hidden = Number(el.dataset.step) !== obStep));
  $$("#ob-dots li").forEach((li, i) => li.classList.toggle("on", i === obStep));
  $("#ob-dots").hidden = obConsentOnly;
  $("#ob-back").hidden = obStep === 0;
  $("#ob-skip").hidden = obStep === 0;
  const p = $("#ob-primary");
  p.hidden = obStep === 1 && obMode === "artist"; // artists are picked straight from the list
  p.textContent = obStep === 0 ? t(obConsentOnly ? "ob.continue" : "ob.start") : t("ob.go");
  p.disabled = obStep === 0 && !$("#ob-agree").checked;
}
function obGo(n) {
  obStep = n;
  obRender();
  if (n === 1) (obMode === "artist" ? $("#ob-q") : $("#ob-in")).focus();
}
$("#ob-agree").addEventListener("change", obRender);
function showOnboarding() {
  obConsentOnly = S.settings.onboarded; // existing users only need to accept the documents
  $("#ob-dir").textContent = S.settings.output_dir;
  $("#onboard").hidden = false;
  obRender();
}
let obSeq = 0, obTimer;
async function obSearch() {
  const term = $("#ob-q").value.trim(), out = $("#ob-results"), seq = ++obSeq;
  if (!term) return (out.innerHTML = "");
  out.innerHTML = `<div class="ob-note muted">${t("search.searching")}</div>`;
  let res = [];
  try { res = await invoke("search_catalog", { kind: "artists", term, provider: null }); }
  catch (e) { if (seq === obSeq) out.innerHTML = `<div class="ob-note" style="color:var(--bad)">${esc(errText(e))}</div>`; return; }
  if (seq !== obSeq) return;
  out.innerHTML = res.length
    ? res.slice(0, 8).map((a) => `<div class="row click" data-ob-artist="${a.id}">
        <div class="avatar">${esc(initials(a.name))}</div>
        <div class="grow"><div class="t">${esc(a.name)}</div><div class="s">${esc(a.genre)}</div></div><span class="muted">›</span></div>`).join("")
    : `<div class="ob-note muted">${t("ob.noartists")}</div>`;
}
$("#ob-q").addEventListener("input", () => { clearTimeout(obTimer); obTimer = setTimeout(obSearch, 300); });
$("#ob-results").addEventListener("click", (e) => {
  const r = e.target.closest("[data-ob-artist]");
  if (!r) return;
  finishOnboarding();
  setTab("search");
  openArtist(Number(r.dataset.obArtist));
});
$("#ob-seg").addEventListener("click", (e) => {
  const b = e.target.closest("button"); if (!b) return;
  $$("#ob-seg button").forEach((x) => x.classList.toggle("on", x === b));
  obMode = b.dataset.v;
  $("#ob-artist").hidden = obMode !== "artist";
  $("#ob-link").hidden = obMode !== "link";
  obRender();
  (obMode === "artist" ? $("#ob-q") : $("#ob-in")).focus();
});
function obFetch() {
  const text = $("#ob-in").value;
  if (!text.trim()) return toast(t("link.paste"));
  finishOnboarding();
  setTab("link");
  $("#link-in").value = text;
  fetchLinks();
}
$("#ob-primary").addEventListener("click", () => {
  if (obStep !== 0) return obFetch();
  if (!$("#ob-agree").checked) return;
  S.settings.accepted_terms = true;
  if (obConsentOnly) return finishOnboarding();
  saveSettings();
  obGo(1);
});
$("#ob-back").addEventListener("click", () => obGo(0));
$("#ob-in").addEventListener("keydown", (e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); obFetch(); } });
$("#ob-skip").addEventListener("click", finishOnboarding);
$("#ob-change").addEventListener("click", async () => {
  const dir = await window.__TAURI__.dialog.open({ directory: true, defaultPath: S.settings.output_dir });
  if (dir) { S.settings.output_dir = dir; $("#s-dir").value = dir; $("#ob-dir").textContent = dir; saveSettings(); }
});
document.addEventListener("keydown", (e) => { if (e.key === "Escape" && !$("#onboard").hidden && S.settings.accepted_terms) finishOnboarding(); });

// ---------- boot ----------
(async function init() {
  S.settings = await invoke("get_settings");
  LANG = DICT[S.settings.language] ? S.settings.language : detectLang();
  loadSettingsUi();
  applyI18n();
  fillLangSelect();
  renderMenu();
  setPane("downloads");
  renderWinControls();
  syncWinState();
  document.fonts?.ready.then(movePill);
  movePill();
  syncSwitchers();
  (await invoke("list_jobs")).forEach((v) => S.jobs.set(v.id, v));
  rebuildJobs();
  renderSearch();
  refreshTools();
  if (!S.settings.onboarded || !S.settings.accepted_terms) showOnboarding();
  else $("#q").focus();
})();

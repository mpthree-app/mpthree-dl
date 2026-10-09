(function () {
  var REPO = "mpthree-app/mpthree-dl";
  var root = document.documentElement;

  // language toggle (pre-set by inline head script)
  var btn = document.getElementById("lang");
  if (btn) btn.addEventListener("click", function () {
    var next = root.lang === "ru" ? "en" : "ru";
    root.lang = next;
    try { localStorage.setItem("lang", next); } catch (e) {}
  });

  if (window.lucide) lucide.createIcons();

  function esc(s) { var d = document.createElement("div"); d.textContent = s == null ? "" : s; return d.innerHTML; }
  function ic(name) { return '<i data-lucide="' + name + '" class="i"></i>'; }
  function bi(en, ru) { return '<span class="en">' + en + '</span><span class="ru">' + ru + "</span>"; }
  function icons() { if (window.lucide) lucide.createIcons(); }
  function num(n) { return n >= 1000 ? (n / 1000).toFixed(1).replace(/\.0$/, "") + "k" : String(n); }

  // GitHub widget
  var gh = document.getElementById("gh-stats");
  if (gh) fetch("https://api.github.com/repos/" + REPO)
    .then(function (r) { if (!r.ok) throw 0; return r.json(); })
    .then(function (d) {
      var s = [
        '<a class="stat" href="' + d.html_url + '/stargazers">' + ic("star") + "<b>" + num(d.stargazers_count) + "</b></a>",
        '<a class="stat" href="' + d.html_url + '/forks">' + ic("git-fork") + "<b>" + num(d.forks_count) + "</b></a>",
        '<a class="stat" href="' + d.html_url + '/issues">' + ic("circle-dot") + "<b>" + num(d.open_issues_count) + "</b> " + bi("issues", "issues") + "</a>"
      ];
      if (d.license && d.license.spdx_id) s.push('<span class="stat">' + ic("scale") + "<b>" + esc(d.license.spdx_id) + "</b></span>");
      if (d.language) s.push('<span class="stat">' + ic("code-xml") + "<b>" + esc(d.language) + "</b></span>");
      if (d.pushed_at) s.push('<span class="stat">' + ic("clock") + bi("updated", "обновлено") + " <b>" + new Date(d.pushed_at).toLocaleDateString() + "</b></span>");
      gh.innerHTML = s.join("");
      icons();
    })
    .catch(function () { gh.innerHTML = '<a class="stat" href="https://github.com/' + REPO + '">' + ic("github") + bi("Open on GitHub", "Открыть на GitHub") + "</a>"; icons(); });

  // Releases
  var el = document.getElementById("releases");
  if (!el) return;
  function size(b) { return (b / 1048576).toFixed(1) + " MB"; }
  function none() {
    el.innerHTML = '<div class="card">' + bi("No release has been published yet. Check <a href=\"https://github.com/" + REPO + "/releases\">GitHub releases</a> soon, or build from source.",
      "Релиз ещё не опубликован. Загляните на <a href=\"https://github.com/" + REPO + "/releases\">GitHub releases</a> позже или соберите из исходников.") + "</div>";
  }
  fetch("https://api.github.com/repos/" + REPO + "/releases?per_page=5")
    .then(function (r) { if (!r.ok) throw 0; return r.json(); })
    .then(function (list) {
      list = list.filter(function (r) { return !r.draft; });
      if (!list.length) return none();
      el.innerHTML = list.map(function (r, i) {
        var assets = (r.assets || []).map(function (a) {
          return '<a href="' + esc(a.browser_download_url) + '">' + ic("download") + esc(a.name) + ' <span class="meta">' + size(a.size) + "</span></a>";
        }).join("");
        var tag = r.prerelease ? '<span class="tag pre">pre-release</span>' : (i === 0 ? '<span class="tag">' + bi("latest", "последний") + "</span>" : "");
        return '<div class="card rel"><h3>' + esc(r.name || r.tag_name) + tag + "</h3>" +
          '<div class="meta">' + esc(r.tag_name) + " · " + new Date(r.published_at).toLocaleDateString() + ' · <a href="' + esc(r.html_url) + '">' + bi("release notes", "описание релиза") + "</a></div>" +
          '<div class="assets">' + (assets || '<span class="meta">' + bi("No files attached.", "Файлов нет.") + "</span>") + "</div></div>";
      }).join("");
      icons();
    })
    .catch(function () {
      el.innerHTML = '<div class="card">' + bi("Releases could not be loaded. See <a href=\"https://github.com/" + REPO + "/releases\">all releases on GitHub</a>.",
        "Не удалось загрузить релизы. Смотрите <a href=\"https://github.com/" + REPO + "/releases\">все релизы на GitHub</a>.") + "</div>";
    });
})();

// code-graph viewer: pan, zoom, search, click-to-focus, community legend. No dependencies, no network.
(function () {
  var D = JSON.parse(document.getElementById("d").textContent);
  var svg = document.getElementById("g");
  var N = svg.querySelectorAll("#nodes circle");
  var E = svg.querySelectorAll("#edges line");
  var HL = document.getElementById("hl");
  var info = document.getElementById("info");
  var q = document.getElementById("q");
  var adj = D.n.map(function () { return []; });
  D.e.forEach(function (e, i) { adj[e[0]].push(i); adj[e[1]].push(i); });
  var vb = D.v.slice();
  var sel = -1;

  function view() { svg.setAttribute("viewBox", vb.join(" ")); }
  function clear() {
    svg.classList.remove("f");
    svg.querySelectorAll(".on,.pick").forEach(function (x) { x.classList.remove("on", "pick"); });
    while (HL.firstChild) HL.removeChild(HL.firstChild);
  }
  function label(i) {
    var c = N[i];
    var t = document.createElementNS(svg.namespaceURI, "text");
    t.setAttribute("x", +c.getAttribute("cx") + 7);
    t.setAttribute("y", +c.getAttribute("cy") + 4);
    t.textContent = D.n[i][0];
    HL.appendChild(t);
  }
  function show(ids, edges, picked) {
    clear();
    if (!ids.length) return;
    svg.classList.add("f");
    ids.forEach(function (i) { N[i].classList.add("on"); });
    edges.forEach(function (i) { E[i].classList.add("on"); });
    picked.slice(0, 80).forEach(function (i) { N[i].classList.add("pick"); label(i); });
  }
  function row(text, cls) {
    var d = document.createElement("div");
    if (cls) d.className = cls;
    d.textContent = text;
    info.appendChild(d);
  }
  function select(i) {
    sel = i;
    var ed = adj[i], ids = [i];
    ed.forEach(function (k) { ids.push(D.e[k][0] === i ? D.e[k][1] : D.e[k][0]); });
    show(ids, ed, [i].concat(ids.slice(1)));
    info.textContent = "";
    var n = D.n[i];
    row(n[0], "t");
    row(n[1] + (n[2] ? " " + n[2] : ""));
    if (n[7]) row("label: " + n[7]);
    row("community: " + n[4]);
    if (n[5]) {
      var a = document.createElement("a");
      a.setAttribute("href", n[5]);
      a.textContent = "Open in " + n[4] + " \u2192";
      info.appendChild(a);
    }
    ed.slice(0, 60).forEach(function (k) {
      var e = D.e[k], out = e[0] === i, o = D.n[out ? e[1] : e[0]];
      row((out ? "→ " : "← ") + o[0] + "  [" + e[2] + "]");
    });
    if (ed.length > 60) row("… " + (ed.length - 60) + " more");
  }
  function reset() {
    sel = -1;
    clear();
    info.textContent = "Click a node.";
  }
  function search() {
    var t = q.value.trim().toLowerCase();
    if (!t) return reset();
    var hit = [];
    D.n.forEach(function (n, i) { if (n[0].toLowerCase().indexOf(t) >= 0 || n[1].toLowerCase().indexOf(t) >= 0) hit.push(i); });
    show(hit, [], hit);
    info.textContent = hit.length + " match" + (hit.length === 1 ? "" : "es");
  }
  function community(c) {
    var ids = [];
    D.n.forEach(function (n, i) { if (n[3] === c) ids.push(i); });
    var es = [];
    D.e.forEach(function (e, k) { if (D.n[e[0]][3] === c && D.n[e[1]][3] === c) es.push(k); });
    var top = ids.slice().sort(function (a, b) { return adj[b].length - adj[a].length; });
    show(ids, es, top.slice(0, 25));
    info.textContent = "";
    row("community " + c + ": " + D.n[ids[0]][4], "t");
    row(ids.length + " nodes");
  }
  q.addEventListener("input", search);
  document.querySelectorAll("#legend li").forEach(function (li) {
    li.addEventListener("click", function () { community(+li.getAttribute("data-c")); });
  });

  // pan and zoom
  function pt(ev) {
    var r = svg.getBoundingClientRect();
    return [vb[0] + (ev.clientX - r.left) * vb[2] / r.width, vb[1] + (ev.clientY - r.top) * vb[3] / r.height];
  }
  svg.addEventListener("wheel", function (ev) {
    ev.preventDefault();
    var p = pt(ev), k = ev.deltaY < 0 ? 0.85 : 1 / 0.85;
    vb = [p[0] - (p[0] - vb[0]) * k, p[1] - (p[1] - vb[1]) * k, vb[2] * k, vb[3] * k];
    view();
  }, { passive: false });
  var drag = null, moved = 0;
  svg.addEventListener("pointerdown", function (ev) {
    drag = { x: ev.clientX, y: ev.clientY, vb: vb.slice() };
    moved = 0;
    svg.setPointerCapture(ev.pointerId);
  });
  svg.addEventListener("pointermove", function (ev) {
    if (!drag) return;
    var r = svg.getBoundingClientRect(), dx = ev.clientX - drag.x, dy = ev.clientY - drag.y;
    moved = Math.max(moved, Math.abs(dx) + Math.abs(dy));
    vb = [drag.vb[0] - dx * vb[2] / r.width, drag.vb[1] - dy * vb[3] / r.height, vb[2], vb[3]];
    view();
  });
  svg.addEventListener("pointerup", function (ev) {
    drag = null;
    if (moved > 4) return;
    var t = ev.target;
    if (t.tagName === "circle") {
      var i = Array.prototype.indexOf.call(N, t);
      // a node of another repository: its page, focused on that node
      if (t.classList.contains("r") && D.n[i][5]) location.assign(D.n[i][5]);
      else select(i);
    } else reset();
  });
  document.addEventListener("keydown", function (ev) { if (ev.key === "Escape") { q.value = ""; reset(); } });
  document.getElementById("fit").addEventListener("click", function () { vb = D.v.slice(); view(); });
  // #node=<id>: select that node and centre the view on it (a link from another repository's graph)
  function focus() {
    var m = /^#node=(.*)$/.exec(location.hash), id;
    if (!m) return;
    try { id = decodeURIComponent(m[1]); } catch (_) { return; }
    for (var i = 0; i < D.n.length; i++) {
      if (D.n[i][6] === id) {
        var w = D.v[2] * 0.5, h = D.v[3] * 0.5;
        vb = [+N[i].getAttribute("cx") - w / 2, +N[i].getAttribute("cy") - h / 2, w, h];
        view();
        return select(i);
      }
    }
  }
  window.addEventListener("hashchange", focus);
  reset();
  focus();
})();

// Knowledge graph loader: fetch nodes/edges from the server and render them with
// Cytoscape. The server returns { success, nodes:[{id,label,kind}],
// edges:[{from,to,rel}], error }. On failure we show the error HTML inline.
(function () {
  const mount = document.getElementById("cy");
  if (!mount || typeof cytoscape === "undefined") return;

  fetch("/knowledge/data")
    .then((r) => r.json())
    .then((g) => {
      if (!g.success && g.error) {
        mount.innerHTML = g.error;
        return;
      }
      const elements = [];
      for (const n of g.nodes || []) {
        elements.push({ data: { id: n.id, label: n.label || n.id, kind: n.kind || "" } });
      }
      for (const e of g.edges || []) {
        elements.push({ data: { source: e.from, target: e.to, rel: e.rel || "" } });
      }
      cytoscape({
        container: mount,
        elements,
        style: [
          { selector: "node", style: { "label": "data(label)", "font-size": "9px",
            "background-color": "#5b8def", "color": "#dfe6f3", "width": 18, "height": 18 } },
          { selector: "edge", style: { "width": 1, "line-color": "#3a4358",
            "target-arrow-color": "#3a4358", "target-arrow-shape": "triangle",
            "curve-style": "bezier" } },
        ],
        layout: { name: "cose", animate: false, nodeRepulsion: 8000 },
      });
    })
    .catch((err) => {
      mount.textContent = "Failed to load graph: " + err;
    });
})();

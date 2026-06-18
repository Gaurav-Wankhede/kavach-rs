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
      const cy = cytoscape({
        container: mount,
        elements,
        style: [
          { selector: "node", style: { "label": "data(label)", "font-size": "9px",
            "background-color": "#5b8def", "color": "#dfe6f3", "width": 18, "height": 18 } },
          { selector: "edge", style: { "width": 1, "line-color": "#3a4358",
            "target-arrow-color": "#3a4358", "target-arrow-shape": "triangle",
            "curve-style": "bezier" } },
          // Edge-traversal highlight: the tapped node, its incident edges, and
          // its 1-hop neighbors light up; everything else dims (G4 click-traverse).
          { selector: ".faded", style: { "opacity": 0.12 } },
          { selector: "node.focus", style: { "background-color": "#f2b134", "width": 24, "height": 24 } },
          { selector: "edge.focus", style: { "line-color": "#f2b134",
            "target-arrow-color": "#f2b134", "width": 2 } },
        ],
        layout: { name: "cose", animate: false, nodeRepulsion: 8000 },
      });

      // Click a node -> traverse one hop: spotlight it + its edges + neighbors.
      cy.on("tap", "node", (evt) => {
        const node = evt.target;
        const hood = node.closedNeighborhood(); // the node, its edges, its neighbors
        cy.elements().addClass("faded").removeClass("focus");
        hood.removeClass("faded");
        node.addClass("focus");
        hood.edges().addClass("focus");
      });
      // Tap the empty background -> clear the traversal spotlight.
      cy.on("tap", (evt) => {
        if (evt.target === cy) {
          cy.elements().removeClass("faded focus");
        }
      });
    })
    .catch((err) => {
      mount.textContent = "Failed to load graph: " + err;
    });
})();

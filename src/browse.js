(() => {
  'use strict';
  const reload = document.querySelector('[data-reload]');
  if (reload) reload.addEventListener('click', async () => {
    reload.disabled = true;
    const message = document.querySelector('[data-reload-result]');
    try {
      const response = await fetch('/api/reload', { method: 'POST', headers: { 'X-Belay-Nonce': document.body.dataset.reloadNonce } });
      const data = await response.json();
      message.textContent = data.message;
      if (response.ok) window.location.reload();
    } catch (_) { message.textContent = 'Reload failed; the previous snapshot remains active.'; }
    finally { reload.disabled = false; }
  });

  const graph = document.getElementById('graph');
  if (!graph || typeof cytoscape !== 'function') return;
  const focus = document.body.dataset.focus;
  const cy = cytoscape({
    container: graph,
    elements: [],
    minZoom: 0.25,
    maxZoom: 1.4,
    style: [
      { selector: 'node', style: { label: 'data(label)', width: 44, height: 44, 'font-size': 11, 'font-weight': 600, 'text-wrap': 'wrap', 'text-max-width': 150, 'background-color': '#77869d', color: '#172033', 'text-outline-color': '#fff', 'text-outline-width': 2 } },
      { selector: 'node[entry_type = "goal"]', style: { 'background-color': '#2bb8a5' } },
      { selector: 'node[entry_type = "plan"]', style: { 'background-color': '#5e8cff' } },
      { selector: 'node[entry_type = "decision"]', style: { 'background-color': '#e59a36' } },
      { selector: 'node[entry_type = "work"]', style: { 'background-color': '#a277e8' } },
      { selector: 'node[kind = "evidence"]', style: { 'background-color': '#e1667a', shape: 'diamond' } },
      { selector: 'node[kind = "commit"]', style: { 'background-color': '#596579', shape: 'hexagon' } },
      { selector: 'node[kind = "file"]', style: { 'background-color': '#b8794d', shape: 'round-rectangle' } },
      { selector: 'edge', style: { label: 'data(label)', width: 2, 'curve-style': 'bezier', 'target-arrow-shape': 'triangle', 'font-size': 8, 'line-color': '#7a879c', 'target-arrow-color': '#7a879c' } }
    ],
    layout: { name: 'cose', animate: false }
  });
  const loaded = new Set();
  async function expand(id) {
    if (loaded.has(id)) return;
    const response = await fetch('/api/explore?focus=' + encodeURIComponent(id));
    if (!response.ok) return;
    const data = await response.json();
    cy.add(data.nodes.filter(n => cy.getElementById(n.data.id).empty()));
    cy.add(data.edges.filter(e => cy.getElementById(e.data.id).empty()));
    loaded.add(id);
    if (data.truncated) {
      graph.setAttribute('aria-description', 'Graph neighborhood truncated at the configured safety limit.');
      let notice = document.querySelector('[data-graph-limit]');
      if (!notice) {
        notice = document.createElement('p');
        notice.className = 'warning';
        notice.dataset.graphLimit = 'true';
        graph.insertAdjacentElement('beforebegin', notice);
      }
      notice.textContent = 'Graph neighborhood truncated at the configured safety limit.';
    }
    cy.layout({ name: 'cose', animate: false }).run();
  }
  cy.on('tap', 'node', event => expand(event.target.id()));
  cy.on('dbltap', 'node', event => { const href = event.target.data('href'); if (href) window.location.assign(href); });
  if (focus) expand(focus);
})();

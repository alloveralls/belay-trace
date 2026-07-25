(() => {
	"use strict";
	const reload = document.querySelector("[data-reload]");
	if (reload)
		reload.addEventListener("click", async () => {
			reload.disabled = true;
			const message = document.querySelector("[data-reload-result]");
			try {
				const response = await fetch("/api/reload", {
					method: "POST",
					headers: { "X-Belay-Nonce": document.body.dataset.reloadNonce },
				});
				const data = await response.json();
				message.textContent = data.message;
				if (response.ok) window.location.reload();
			} catch (_) {
				message.textContent =
					"Reload failed; the previous snapshot remains active.";
			} finally {
				reload.disabled = false;
			}
		});

	const graph = document.getElementById("graph");
	if (!graph || typeof cytoscape !== "function") return;
	const focus = document.body.dataset.focus;
	function graphLayout() {
		return {
			name: "cose",
			animate: false,
			idealEdgeLength: 36,
			nodeRepulsion: 768,
			componentSpacing: 45,
			nodeDimensionsIncludeLabels: true,
		};
	}
	const cy = cytoscape({
		container: graph,
		elements: [],
		minZoom: 0.25,
		maxZoom: 1.4,
		style: [
			{
				selector: "node",
				style: {
					label: "data(label)",
					width: 44,
					height: 44,
					"font-size": 11,
					"font-weight": 600,
					"text-wrap": "wrap",
					"text-max-width": 150,
					"background-color": "#77869d",
					color: "#172033",
					"text-outline-color": "#fff",
					"text-outline-width": 2,
				},
			},
			{
				selector: 'node[entry_type = "goal"]',
				style: { "background-color": "#2bb8a5" },
			},
			{
				selector: 'node[entry_type = "plan"]',
				style: { "background-color": "#5e8cff" },
			},
			{
				selector: 'node[entry_type = "decision"]',
				style: { "background-color": "#e59a36" },
			},
			{
				selector: 'node[entry_type = "work"]',
				style: { "background-color": "#a277e8" },
			},
			{
				selector: 'node[kind = "evidence"]',
				style: { "background-color": "#e1667a", shape: "diamond" },
			},
			{
				selector: 'node[kind = "commit"]',
				style: { "background-color": "#596579", shape: "hexagon" },
			},
			{
				selector: 'node[kind = "file"]',
				style: { "background-color": "#b8794d", shape: "round-rectangle" },
			},
			{
				selector: "edge",
				style: {
					label: "data(label)",
					width: 2,
					"curve-style": "bezier",
					"target-arrow-shape": "triangle",
					"font-size": 8,
					"line-color": "#7a879c",
					"target-arrow-color": "#7a879c",
				},
			},
		],
		layout: graphLayout(),
	});
	const loaded = new Set();
	const loading = new Set();
	async function expand(id) {
		if (loaded.has(id) || loading.has(id)) return;
		loading.add(id);
		graph.setAttribute("aria-busy", "true");
		try {
			const response = await fetch(
				"/api/explore?focus=" + encodeURIComponent(id),
			);
			if (!response.ok) return;
			const data = await response.json();
			cy.add(data.nodes.filter((n) => cy.getElementById(n.data.id).empty()));
			cy.add(data.edges.filter((e) => cy.getElementById(e.data.id).empty()));
			loaded.add(id);
			if (data.truncated) {
				graph.setAttribute(
					"aria-description",
					"Graph neighborhood truncated at the configured safety limit.",
				);
				let notice = document.querySelector("[data-graph-limit]");
				if (!notice) {
					notice = document.createElement("p");
					notice.className = "warning";
					notice.dataset.graphLimit = "true";
					graph.insertAdjacentElement("beforebegin", notice);
				}
				notice.textContent =
					"Graph neighborhood truncated at the configured safety limit.";
			}
			cy.layout(graphLayout()).run();
		} finally {
			loading.delete(id);
			if (loading.size === 0) graph.setAttribute("aria-busy", "false");
		}
	}
	// Match Cytoscape's own double-click window so a possible dbltap can cancel
	// the deferred single-node activation before expansion starts.
	const singleTapDelayMs = cy.multiClickDebounceTime();
	const pendingNodeTaps = new Map();
	function cancelPendingNodeTap(id) {
		const timer = pendingNodeTaps.get(id);
		if (timer === undefined) return;
		clearTimeout(timer);
		pendingNodeTaps.delete(id);
	}
	cy.on("tap", "node", (event) => {
		const id = event.target.id();
		cancelPendingNodeTap(id);
		const timer = setTimeout(() => {
			if (pendingNodeTaps.get(id) !== timer) return;
			pendingNodeTaps.delete(id);
			expand(id);
		}, singleTapDelayMs);
		pendingNodeTaps.set(id, timer);
	});
	cy.on("dbltap", "node", (event) => {
		const id = event.target.id();
		cancelPendingNodeTap(id);
		const href = event.target.data("href");
		if (href) window.location.assign(href);
	});
	window.addEventListener("pagehide", () => {
		for (const timer of pendingNodeTaps.values()) clearTimeout(timer);
		pendingNodeTaps.clear();
	});
	if (focus) expand(focus);
})();

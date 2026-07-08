/* <wiki-map>: progressive enhancement for the map popover. The element
   wraps the static tile image; once it becomes visible (popover open)
   it loads the self-hosted Leaflet and swaps in an interactive map on
   Wikimedia tiles. A `source` attribute naming a Commons map-data page
   (P3896 geoshape) is fetched and drawn as an outline. If anything
   fails - file:// output, offline, old browser - the static image
   simply stays. */
(() => {
	const LEAFLET_JS = "/static/leaflet.js";
	const LEAFLET_CSS = "/static/leaflet.css";
	const TILES = "https://maps.wikimedia.org/osm-intl/{z}/{x}/{y}{r}.png";
	const ATTRIBUTION =
		'<a href="https://wikimediafoundation.org/wiki/Maps_Terms_of_Use">Wikimedia maps</a> | ' +
		'Map data &copy; <a href="https://openstreetmap.org/copyright">OpenStreetMap contributors</a>';

	const addScript = (src) =>
		new Promise((resolve, reject) => {
			const existing = document.querySelector(`script[src="${src}"]`);
			if (existing) {
				if (window.L) {
					resolve();
				} else {
					existing.addEventListener("load", resolve);
					existing.addEventListener("error", reject);
				}
				return;
			}
			const el = document.createElement("script");
			el.src = src;
			el.addEventListener("load", resolve);
			el.addEventListener("error", reject);
			document.body.append(el);
		});

	const addStyle = (src) =>
		new Promise((resolve, reject) => {
			if (document.querySelector(`link[href="${src}"]`)) {
				resolve();
				return;
			}
			const el = document.createElement("link");
			el.rel = "stylesheet";
			el.href = src;
			el.addEventListener("load", resolve);
			el.addEventListener("error", reject);
			document.head.append(el);
		});

	class WikiMap extends HTMLElement {
		connectedCallback() {
			this.upgraded = false;
			this.observer = new IntersectionObserver((entries) => {
				if (entries.some((entry) => entry.isIntersecting)) {
					this.observer.disconnect();
					this.upgrade().catch(() => {
						/* static image stays */
					});
				}
			});
			this.observer.observe(this);
		}

		disconnectedCallback() {
			this.observer?.disconnect();
		}

		async upgrade() {
			if (this.upgraded) return;
			this.upgraded = true;

			const latitude = Number.parseFloat(this.getAttribute("latitude"));
			const longitude = Number.parseFloat(this.getAttribute("longitude"));
			const zoom = Number.parseInt(this.getAttribute("zoom"), 10) || 10;
			// fetch the outline before touching the DOM, so a failure
			// still leaves the static image in place
			const geoJSON = await this.fetchGeoShape();

			await addStyle(LEAFLET_CSS);
			await addScript(LEAFLET_JS);

			const container = document.createElement("div");
			container.className = "wiki-map__view";
			this.replaceChildren(container);

			const map = window.L.map(container, {
				center: [latitude, longitude],
				zoom,
			});
			window.L.tileLayer(TILES, {
				maxZoom: 18,
				detectRetina: true,
				attribution: ATTRIBUTION,
			}).addTo(map);
			window.L.control.scale().addTo(map);

			if (geoJSON) {
				const outline = window.L.geoJSON(geoJSON).addTo(map);
				map.fitBounds(outline.getBounds(), { padding: [16, 16] });
			} else {
				window.L.circleMarker([latitude, longitude], { radius: 6 }).addTo(
					map,
				);
			}
		}

		/* The source attribute is a Commons map-data URL as stored on
		   Wikidata (…/data/main/Data:X.map). The page content is JSON
		   whose `data` member is the GeoJSON. */
		async fetchGeoShape() {
			const source = this.getAttribute("source");
			if (!source) return null;
			try {
				const url = new URL(source.replace("/data/main/", "/wiki/"));
				// Wikidata stores spaces in map-data titles as "+"
				const title = decodeURIComponent(
					url.pathname.split("/wiki/").pop(),
				).replaceAll("+", " ");
				const api = `https://${url.hostname}/w/api.php?action=query&prop=revisions&rvprop=content&titles=${encodeURIComponent(title)}&format=json&formatversion=2&origin=*`;
				const response = await fetch(api);
				if (!response.ok) return null;
				const data = await response.json();
				const page = JSON.parse(data.query.pages[0].revisions[0].content);
				return page.data;
			} catch {
				return null;
			}
		}
	}

	customElements.define("wiki-map", WikiMap);
})();

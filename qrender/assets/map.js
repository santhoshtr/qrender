// The qr-map element: the map of a dialog, which the reader moves and
// zooms, drawn by Leaflet with the tiles of Wikimedia maps in the page
// language. Progressive enhancement only: the element holds the static map
// of the page, and without this script or without Leaflet it stays. The
// server gives the data (the pin, the box to show first, the markers, the
// outlines); the script draws it, and CSS gives the colours by the class
// names. Leaflet loads once, when the first map dialog opens: a map needs
// the size of its box, and a closed dialog has none.
// One function scope: the page scripts share one script element.
( () => {
	const root = document.documentElement;
	// The design base is on a page that the server sends (not in a file).
	const base = root.dataset.design;
	if ( !base || !( 'customElements' in window ) ) {
		return;
	}
	const tiles = `https://maps.wikimedia.org/osm-intl/{z}/{x}/{y}{r}.png?lang=${ encodeURIComponent( root.lang ) }`;
	const load = ( element ) => new Promise( ( resolve, reject ) => {
		element.onload = resolve;
		element.onerror = reject;
		document.head.append( element );
	} );
	let loading;
	const leaflet = () => loading ??= Promise.all( [
		load( Object.assign( document.createElement( 'link' ), { rel: 'stylesheet', href: `${ base }/leaflet.css` } ) ),
		load( Object.assign( document.createElement( 'script' ), { src: `${ base }/leaflet.js` } ) )
	] ).then( () => window.L );

	customElements.define( 'qr-map', class extends HTMLElement {
		connectedCallback() {
			this.dialog = this.closest( 'dialog' );
			this.dialog?.addEventListener( 'toggle', ( event ) => {
				if ( event.newState === 'open' ) {
					this.open();
				}
			} );
		}

		async open() {
			if ( this.map ) {
				// The window can have changed since the last open.
				this.map.invalidateSize();
				return;
			}
			const L = await leaflet().catch( () => null );
			// The dialog can have closed while Leaflet loaded: the next open draws.
			if ( !L || this.map || !this.dialog.open ) {
				return;
			}
			const data = JSON.parse( this.dataset.map );
			const icon = ( className ) => L.divIcon( { className, iconSize: null } );
			const attribution = Object.assign( document.createElement( 'a' ), {
				href: 'https://www.openstreetmap.org/copyright',
				textContent: this.dataset.attribution
			} ).outerHTML;

			this.replaceChildren();
			const map = this.map = L.map( this );
			map.attributionControl.setPrefix( false );
			map.fitBounds( data.bounds );
			L.tileLayer( tiles, { maxZoom: 18, attribution } ).addTo( map );
			for ( const outline of data.outlines ) {
				const className = outline.main ? 'main' : 'other';
				if ( outline.rings.length ) {
					L.polygon( outline.rings, { className, interactive: false } ).addTo( map );
				}
				if ( outline.lines.length ) {
					L.polyline( outline.lines, { className: `${ className } open`, interactive: false } ).addTo( map );
				}
			}
			for ( const marker of data.markers ) {
				L.marker( marker.at, { icon: icon( 'marker' ), title: marker.label, keyboard: false } ).addTo( map );
			}
			L.marker( data.pin, { icon: icon( 'pin' ), interactive: false, keyboard: false } ).addTo( map );
		}
	} );
} )();

// The qr-section element: a text of the article in another language (the
// page language has no article), which the reader translates into the page
// language. Progressive enhancement only: the server renders the text, and
// every word of the editor in the dialog of the page (#translator); without
// this script the text stays and the button stays hidden. On "Translate",
// the dialog opens: each block of the text (its title, a paragraph, a list,
// the title of a subsection) goes to the machine translation of Wikimedia
// (MinT, by cxserver), a few at a time, and its translation comes in an
// editable row beside it. "Save" mimics a published edit: the section on
// the page shows the translation. Nothing is sent.
// One function scope: the page scripts share one script element.
( () => {
	const dialog = document.getElementById( 'translator' );
	if ( !dialog || !( 'customElements' in window ) ) {
		return;
	}
	const MT = 'https://cxserver.wikimedia.org/v2/translate';
	// Requests at a time: MinT takes about 1.5 s for a paragraph.
	const PARALLEL = 4;
	const BLOCKS = 'p, ul, ol, dl, blockquote';
	// What a translation may hold: the tags of the text, links to the web.
	const ALLOWED = new Set( [ 'P', 'UL', 'OL', 'LI', 'DL', 'DT', 'DD', 'BLOCKQUOTE',
		'H2', 'H3', 'H4', 'H5', 'H6', 'A', 'B', 'I', 'SPAN' ] );
	const rows = dialog.querySelector( '.rows' );
	const status = dialog.querySelector( '.status' );
	// The section in the dialog, and the requests of its translation.
	let current = null;
	let controller = null;

	// The blocks of a text, in order: a subsection gives its title, then
	// its blocks.
	const units = ( container ) => [ ...container.children ].flatMap( ( el ) => {
		if ( el.matches( BLOCKS ) ) {
			return [ el ];
		}
		if ( el.matches( 'details' ) ) {
			return [ el.querySelector( ':scope > summary > :is( h3, h4, h5, h6 )' ), ...units( el ) ];
		}
		return [];
	} );

	// A block without the glosses of its links (the labels in the page
	// language) and the marks of its notes: the machine translation reads
	// the source text only.
	const source = ( el ) => {
		const copy = el.cloneNode( true );
		copy.querySelectorAll( 'sup.ref' ).forEach( ( mark ) => mark.remove() );
		copy.querySelectorAll( 'rt' ).forEach( ( rt ) => rt.remove() );
		copy.querySelectorAll( 'ruby' ).forEach( ( ruby ) => ruby.replaceWith( ...ruby.childNodes ) );
		return copy;
	};

	// Untrusted HTML of the service, cleaned: an element that is not
	// allowed leaves its text; no attribute but an http(s) link.
	const clean = ( html ) => {
		const template = document.createElement( 'template' );
		template.innerHTML = html;
		const walk = ( node ) => {
			for ( const child of [ ...node.children ] ) {
				walk( child );
				if ( !ALLOWED.has( child.tagName ) ) {
					child.replaceWith( ...child.childNodes );
					continue;
				}
				for ( const { name, value } of [ ...child.attributes ] ) {
					if ( !( name === 'href' && /^https?:\/\//.test( value ) ) ) {
						child.removeAttribute( name );
					}
				}
			}
		};
		walk( template.content );
		return template.content;
	};

	const translate = async ( from, to, el, signal ) => {
		// MinT translates the first element only: one block in a section.
		const response = await fetch( `${ MT }/${ from }/${ to }/MinT`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify( { html: `<section>${ el.outerHTML }</section>` } ),
			signal
		} );
		if ( !response.ok ) {
			throw new Error( response.status );
		}
		const { contents } = await response.json();
		const fragment = clean( contents );
		if ( !fragment.textContent.trim() ) {
			throw new Error( 'empty' );
		}
		return fragment;
	};

	// The jobs, `n` at a time.
	const run = async ( jobs, n ) => {
		let next = 0;
		await Promise.all( Array.from( { length: n }, async () => {
			while ( next < jobs.length ) {
				await jobs[ next++ ]();
			}
		} ) );
	};

	// The content of a translated row: the children of its block when it
	// is of the same kind as the source (a paragraph for a paragraph).
	const content = ( target, unit ) => {
		const el = target.firstElementChild;
		return el && el.tagName === unit.tagName && target.children.length === 1 ?
			[ ...el.childNodes ] : [ ...target.childNodes ];
	};

	dialog.addEventListener( 'close', () => {
		controller?.abort();
		current = null;
		rows.replaceChildren();
		status.textContent = '';
	} );
	dialog.querySelector( '.discard' ).addEventListener( 'click', () => dialog.close() );
	dialog.querySelector( '.save' ).addEventListener( 'click', () => {
		current?.save( [ ...rows.querySelectorAll( '.translated' ) ] );
		dialog.close();
	} );

	customElements.define( 'qr-section', class extends HTMLElement {
		connectedCallback() {
			// The section's own button and text, not those of a section in it.
			this.button = this.querySelector( '.translate' );
			this.heading = this.querySelector( ':scope > .head > :is( h2, h3, h4, h5, h6 )' );
			this.body = this.querySelector( ':scope > .body' );
			if ( !this.button || !this.body ) {
				return;
			}
			this.button.hidden = false;
			this.button.addEventListener( 'click', () => this.open() );
		}

		// The blocks on the page: the title first, then the text.
		units() {
			return [ ...( this.heading ? [ this.heading ] : [] ), ...units( this.body ) ];
		}

		async open() {
			if ( dialog.open ) {
				return;
			}
			const { from, to, dir } = this.dataset;
			// The English blocks of the first open: after a save, the page
			// shows the translation.
			this.sources ??= this.units().map( source );
			current = this;
			controller = new AbortController();
			const { signal } = controller;
			dialog.querySelector( '.section-title' ).textContent =
				this.heading ? ` · ${ this.sources[ 0 ].textContent }` : '';
			const jobs = this.sources.map( ( block, i ) => {
				const target = document.createElement( 'div' );
				target.className = 'translated';
				target.lang = to;
				target.dir = dir;
				target.contentEditable = 'true';
				const original = document.createElement( 'div' );
				original.className = 'original';
				original.lang = from;
				original.append( block.cloneNode( true ) );
				rows.append( original, target );
				if ( this.saved ) {
					target.append( this.saved[ i ].cloneNode( true ) );
					return null;
				}
				target.textContent = rows.dataset.translating;
				target.setAttribute( 'aria-busy', 'true' );
				return async () => {
					try {
						target.replaceChildren( await translate( from, to, block, signal ) );
					} catch {
						if ( signal.aborted ) {
							return;
						}
						target.replaceChildren( block.cloneNode( true ) );
						target.lang = from;
						target.classList.add( 'failed' );
						target.title = rows.dataset.failed;
					}
					target.removeAttribute( 'aria-busy' );
				};
			} ).filter( Boolean );
			dialog.showModal();
			if ( jobs.length ) {
				status.textContent = rows.dataset.translating;
				await run( jobs, PARALLEL );
				if ( !signal.aborted ) {
					status.textContent = '';
				}
			}
		}

		// The rows of the dialog, in the place of the blocks on the page:
		// the same elements, their children replaced, so a second edit maps
		// row to block again.
		save( targets ) {
			const { dir } = this.dataset;
			// A row that was not translated keeps the language of its source.
			const langs = targets.map( ( target ) => target.lang );
			this.saved = targets.map( ( target ) => {
				const copy = document.createDocumentFragment();
				copy.append( ...[ ...target.childNodes ].map( ( n ) => n.cloneNode( true ) ) );
				return copy;
			} );
			this.units().forEach( ( unit, i ) => {
				unit.replaceChildren( ...content( this.saved[ i ].cloneNode( true ), unit ) );
				unit.lang = langs[ i ];
				unit.dir = langs[ i ] === this.dataset.to ? dir : '';
			} );
			this.classList.add( 'translated' );
			this.querySelector( '.saved-note' ).hidden = false;
		}
	} );
} )();

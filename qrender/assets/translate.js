// The qr-section element: a text of the article in another language (the
// page language has no article), which the reader translates into the page
// language. Progressive enhancement only: the server renders the text and
// every word of the editor; without this script the text stays and the
// button stays hidden. On "Translate", each block of the text (a paragraph,
// a list, the title of a subsection) goes to the machine translation of
// Wikimedia (MinT, by cxserver), a few at a time, and its translation comes
// in an editable row beside it. "Save" is a prototype: nothing is sent.
// One function scope: the page scripts share one script element.
( () => {
	if ( !( 'customElements' in window ) ) {
		return;
	}
	const MT = 'https://cxserver.wikimedia.org/v2/translate';
	// Requests at a time: MinT takes about 1.5 s for a paragraph.
	const PARALLEL = 4;
	const BLOCKS = 'p, ul, ol, dl, blockquote';
	// What a translation may hold: the tags of the text, links to the web.
	const ALLOWED = new Set( [ 'P', 'UL', 'OL', 'LI', 'DL', 'DT', 'DD', 'BLOCKQUOTE',
		'H3', 'H4', 'H5', 'H6', 'A', 'B', 'I', 'SPAN' ] );

	// The blocks of a text, in order: a subsection gives its title, then
	// its blocks; a collapsed part without a title (the rest of the lead)
	// gives its blocks.
	const units = ( container ) => [ ...container.children ].flatMap( ( el ) => {
		if ( el.matches( BLOCKS ) ) {
			return [ el ];
		}
		if ( el.matches( 'details' ) ) {
			const title = el.querySelector( ':scope > summary > :is( h3, h4, h5, h6 )' );
			const rest = units( el );
			return title ? [ title, ...rest ] : rest;
		}
		return [];
	} );

	// A block without the glosses of its links (the labels in the page
	// language): the machine translation reads the source text only.
	const source = ( el ) => {
		const copy = el.cloneNode( true );
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

	const translate = async ( from, to, el ) => {
		// MinT translates the first element only: one block in a section.
		const response = await fetch( `${ MT }/${ from }/${ to }/MinT`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify( { html: `<section>${ el.outerHTML }</section>` } )
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

	customElements.define( 'qr-section', class extends HTMLElement {
		connectedCallback() {
			// The section's own button and editor, not those of a section in it.
			this.button = this.querySelector( '.translate' );
			this.editor = this.querySelector( ':scope > .translation, :scope > details > .translation' );
			this.body = this.querySelector( ':scope > .body, :scope > details > .body' );
			if ( !this.button || !this.editor || !this.body ) {
				return;
			}
			this.button.hidden = false;
			this.button.addEventListener( 'click', ( event ) => {
				// In a summary, the click would also close the section.
				event.preventDefault();
				this.open();
			} );
			this.editor.querySelector( '.discard' ).addEventListener( 'click', () => this.close() );
			this.editor.querySelector( '.save' ).addEventListener( 'click', () => {
				this.editor.querySelector( '.saved' ).hidden = false;
			} );
		}

		async open() {
			this.querySelector( ':scope > details' )?.setAttribute( 'open', '' );
			if ( !this.editor.hidden ) {
				return;
			}
			const { from, to, dir } = this.dataset;
			const rows = this.editor.querySelector( '.rows' );
			const status = this.editor.querySelector( '.status' );
			const jobs = units( this.body ).map( ( el ) => {
				const block = source( el );
				const target = document.createElement( 'div' );
				target.className = 'translated';
				target.lang = to;
				target.dir = dir;
				target.contentEditable = 'true';
				target.textContent = rows.dataset.translating;
				target.setAttribute( 'aria-busy', 'true' );
				const original = document.createElement( 'div' );
				original.className = 'original';
				original.lang = from;
				original.append( block.cloneNode( true ) );
				rows.append( original, target );
				return async () => {
					try {
						target.replaceChildren( await translate( from, to, block ) );
					} catch {
						target.replaceChildren( block );
						target.lang = from;
						target.classList.add( 'failed' );
						target.title = rows.dataset.failed;
					}
					target.removeAttribute( 'aria-busy' );
				};
			} );
			this.body.hidden = true;
			this.editor.hidden = false;
			this.button.disabled = true;
			status.textContent = rows.dataset.translating;
			await run( jobs, PARALLEL );
			status.textContent = '';
		}

		close() {
			this.editor.hidden = true;
			this.editor.querySelector( '.rows' ).replaceChildren();
			this.editor.querySelector( '.saved' ).hidden = true;
			this.body.hidden = false;
			this.button.disabled = false;
		}
	} );
} )();

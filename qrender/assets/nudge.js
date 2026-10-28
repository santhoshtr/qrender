// The nudges (.nudge): a value that the item lacks, asked for in a dialog
// of its own. Progressive enhancement only: without this script, the
// dialog opens (invoker commands) and its link adds the value on Wikidata.
// A search field lists the items of Wikidata (wbsearchentities) or the
// files of Commons under it. "Save" mimics a published edit: the nudge
// shows the value, saved on this page only. Nothing is sent. Text only
// from the answers: nothing of them is parsed as HTML.
// One function scope: the page scripts share one script element.
( () => {
	const nudges = document.querySelectorAll( '.nudge' );
	if ( !nudges.length ) {
		return;
	}
	const WIKIDATA = 'https://www.wikidata.org/w/api.php';
	const COMMONS = 'https://commons.wikimedia.org/w/api.php';
	const LIMIT = 7;
	// The wait after a key before the request, in milliseconds.
	const DELAY = 200;
	const root = document.documentElement;
	const lang = root.lang;
	const locale = root.dataset.locale || lang;

	const get = async ( api, params, signal ) => {
		const query = new URLSearchParams( { ...params, format: 'json', origin: '*' } );
		const response = await fetch( `${ api }?${ query }`, { signal } );
		return response.ok ? response.json() : {};
	};

	// The results of a text, by source: a value, its text, and a
	// description or a thumbnail.
	const sources = {
		item: async ( text, signal ) => {
			const { search = [] } = await get( WIKIDATA, {
				action: 'wbsearchentities',
				search: text,
				language: lang,
				uselang: lang,
				type: 'item',
				limit: LIMIT
			}, signal );
			return search.map( ( item ) => ( {
				value: item.id,
				text: item.label || item.id,
				description: item.description || ''
			} ) );
		},
		file: async ( text, signal ) => {
			const { query: { pages = {} } = {} } = await get( COMMONS, {
				action: 'query',
				generator: 'search',
				gsrsearch: text,
				gsrnamespace: 6,
				gsrlimit: LIMIT,
				prop: 'imageinfo',
				iiprop: 'url',
				iiurlwidth: 80
			}, signal );
			return Object.values( pages )
				.sort( ( a, b ) => a.index - b.index )
				.map( ( page ) => {
					const name = page.title.replace( /^[^:]+:/, '' );
					return { value: name, text: name, thumb: page.imageinfo?.[ 0 ]?.thumburl };
				} );
		}
	};

	// A search field: a combobox with its list. A text that is not a
	// result chosen from the list is not valid.
	const combobox = ( input ) => {
		const list = document.getElementById( input.getAttribute( 'aria-controls' ) );
		const search = sources[ input.dataset.search ];
		let controller = null;
		let timer = 0;
		// The option that the arrow keys chose; -1 for none.
		let active = -1;

		const options = () => [ ...list.children ];

		const mark = ( index ) => {
			const all = options();
			active = index;
			all.forEach( ( o, i ) => o.setAttribute( 'aria-selected', String( i === index ) ) );
			if ( index < 0 ) {
				input.removeAttribute( 'aria-activedescendant' );
				return;
			}
			input.setAttribute( 'aria-activedescendant', all[ index ].id );
			all[ index ].scrollIntoView( { block: 'nearest' } );
		};

		const show = ( shown ) => {
			list.hidden = !shown;
			input.setAttribute( 'aria-expanded', String( shown ) );
			if ( !shown ) {
				mark( -1 );
			}
		};

		const option = ( result, index ) => {
			const li = document.createElement( 'li' );
			li.id = `${ list.id }-${ index }`;
			li.setAttribute( 'role', 'option' );
			li.setAttribute( 'aria-selected', 'false' );
			li.dataset.value = result.value;
			if ( result.thumb ) {
				const img = document.createElement( 'img' );
				img.src = result.thumb;
				img.alt = '';
				img.loading = 'lazy';
				li.append( img );
			}
			const text = document.createElement( 'span' );
			text.className = 'text';
			text.textContent = result.text;
			li.append( text );
			if ( result.description ) {
				const description = document.createElement( 'span' );
				description.className = 'description';
				description.textContent = result.description;
				li.append( description );
			}
			return li;
		};

		const choose = ( li ) => {
			input.value = li.querySelector( '.text' ).textContent;
			input.dataset.value = li.dataset.value;
			input.setCustomValidity( '' );
			show( false );
		};

		input.addEventListener( 'input', () => {
			clearTimeout( timer );
			delete input.dataset.value;
			const text = input.value.trim();
			input.setCustomValidity( text ? input.dataset.choose : '' );
			if ( !text ) {
				controller?.abort();
				list.replaceChildren();
				show( false );
				return;
			}
			timer = setTimeout( async () => {
				controller?.abort();
				controller = new AbortController();
				try {
					const results = await search( text, controller.signal );
					list.replaceChildren( ...results.map( option ) );
					show( results.length > 0 );
				} catch ( e ) {
					if ( e.name !== 'AbortError' ) {
						list.replaceChildren();
						show( false );
					}
				}
			}, DELAY );
		} );

		input.addEventListener( 'keydown', ( event ) => {
			const count = options().length;
			if ( ( event.key === 'ArrowDown' || event.key === 'ArrowUp' ) && count ) {
				event.preventDefault();
				show( true );
				// From the field to the first option, from the last option
				// back to the field; and up the other way.
				const step = event.key === 'ArrowDown' ? 1 : count;
				mark( ( active + 1 + step ) % ( count + 1 ) - 1 );
			} else if ( event.key === 'Enter' && !list.hidden && active >= 0 ) {
				// Enter chooses the option; it does not save.
				event.preventDefault();
				choose( options()[ active ] );
			} else if ( event.key === 'Escape' && !list.hidden ) {
				// The first Escape closes the list; the next closes the dialog.
				event.preventDefault();
				show( false );
			}
		} );

		input.addEventListener( 'blur', () => show( false ) );
		// A click on an option keeps the focus in the field.
		list.addEventListener( 'mousedown', ( event ) => event.preventDefault() );
		list.addEventListener( 'click', ( event ) => {
			const li = event.target.closest( '[role=option]' );
			if ( li ) {
				choose( li );
			}
		} );
	};

	// The field of a date: a day, or a year only.
	const precision = ( select, input ) => {
		select.addEventListener( 'change', () => {
			input.value = '';
			input.dataset.precision = select.value;
			if ( select.value === 'year' ) {
				input.type = 'number';
				input.step = '1';
			} else {
				input.type = 'date';
			}
		} );
	};

	// The value as the nudge shows it: a number or a date formatted for
	// the page locale, the other fields as they are.
	const shown = ( input ) => {
		const value = input.value.trim();
		if ( input.type === 'number' && input.dataset.precision !== 'year' && value ) {
			return new Intl.NumberFormat( locale ).format( Number( value ) );
		}
		if ( input.type === 'date' && value ) {
			return new Intl.DateTimeFormat( locale, { dateStyle: 'long', timeZone: 'UTC' } )
				.format( new Date( value ) );
		}
		return value;
	};

	for ( const nudge of nudges ) {
		const dialog = nudge.querySelector( 'dialog' );
		dialog.querySelector( '.save' ).hidden = false;
		dialog.querySelectorAll( '[data-search]' ).forEach( combobox );
		const select = dialog.querySelector( '[data-precision]' );
		if ( select ) {
			precision( select, dialog.querySelector( '[data-show]' ) );
		}
		// Save: the nudge shows the value. Close: the fields keep what the
		// reader wrote, for the next time.
		dialog.addEventListener( 'close', () => {
			if ( dialog.returnValue !== 'save' ) {
				return;
			}
			dialog.returnValue = '';
			const value = nudge.querySelector( '.value' );
			value.textContent = [ ...dialog.querySelectorAll( '[data-show]' ) ]
				.map( shown )
				.filter( Boolean )
				.join( ' ' );
			value.hidden = false;
			nudge.querySelector( '.saved' ).hidden = false;
			nudge.classList.add( 'is-saved' );
			nudge.querySelector( '.add' ).textContent = nudge.dataset.edit;
		} );
	}
} )();

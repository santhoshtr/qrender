// The search of the page header (#search): the Wikidata items that match
// the text, in the page language (wbsearchentities), as a list under the
// field. An item opens its page here, in the page language. Progressive
// enhancement only: without this script, the form opens the search of
// Wikidata. Text only from the answer: nothing of it is parsed as HTML.
// One function scope: the page scripts share one script element.
( () => {
	const form = document.getElementById( 'search' );
	if ( !form ) {
		return;
	}
	const API = 'https://www.wikidata.org/w/api.php';
	const LIMIT = 7;
	// The wait after a key before the request, in milliseconds.
	const DELAY = 200;
	const lang = document.documentElement.lang;
	const input = form.querySelector( '[role=combobox]' );
	const list = form.querySelector( '[role=listbox]' );
	let controller = null;
	let timer = 0;
	// The option that the arrow keys chose; -1 for none.
	let active = -1;

	const options = () => [ ...list.children ];

	const choose = ( index ) => {
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
			choose( -1 );
		}
	};

	// An item: its label (its id when it has none) and its description.
	const option = ( item, index ) => {
		const li = document.createElement( 'li' );
		li.id = `search-option-${ index }`;
		li.setAttribute( 'role', 'option' );
		li.setAttribute( 'aria-selected', 'false' );
		const a = document.createElement( 'a' );
		a.href = form.dataset.base + item.id;
		a.tabIndex = -1;
		const label = document.createElement( 'span' );
		label.textContent = item.label || item.id;
		a.append( label );
		if ( item.description ) {
			const description = document.createElement( 'span' );
			description.className = 'description';
			description.textContent = item.description;
			a.append( description );
		}
		li.append( a );
		return li;
	};

	// The items of a text in the list. The number of items; 0 also for a
	// failed request.
	const search = async ( text ) => {
		controller?.abort();
		controller = new AbortController();
		const params = new URLSearchParams( {
			action: 'wbsearchentities',
			search: text,
			language: lang,
			uselang: lang,
			type: 'item',
			limit: LIMIT,
			format: 'json',
			origin: '*'
		} );
		try {
			const response = await fetch( `${ API }?${ params }`, { signal: controller.signal } );
			const { search: items = [] } = response.ok ? await response.json() : {};
			list.replaceChildren( ...items.map( option ) );
			show( items.length > 0 );
			return items.length;
		} catch ( e ) {
			if ( e.name !== 'AbortError' ) {
				list.replaceChildren();
				show( false );
			}
			return 0;
		}
	};

	const open = ( li ) => location.assign( li.querySelector( 'a' ).href );

	input.addEventListener( 'input', () => {
		clearTimeout( timer );
		choose( -1 );
		const text = input.value.trim();
		if ( !text ) {
			controller?.abort();
			list.replaceChildren();
			show( false );
			return;
		}
		timer = setTimeout( () => search( text ), DELAY );
	} );

	input.addEventListener( 'keydown', ( event ) => {
		const count = options().length;
		if ( ( event.key === 'ArrowDown' || event.key === 'ArrowUp' ) && count ) {
			event.preventDefault();
			show( true );
			// From the field to the first option, from the last option
			// back to the field; and up the other way.
			const step = event.key === 'ArrowDown' ? 1 : count;
			choose( ( active + 1 + step ) % ( count + 1 ) - 1 );
		} else if ( event.key === 'Escape' && !list.hidden ) {
			// The first Escape closes the list; the next clears the field.
			event.preventDefault();
			show( false );
		}
	} );

	// Enter: the chosen option, or else the first item of the text. A text
	// whose items have not come yet waits for them.
	form.addEventListener( 'submit', async ( event ) => {
		event.preventDefault();
		const chosen = options()[ active ];
		if ( chosen ) {
			open( chosen );
			return;
		}
		clearTimeout( timer );
		const text = input.value.trim();
		if ( text && await search( text ) ) {
			open( options()[ 0 ] );
		}
	} );

	input.addEventListener( 'focus', () => show( options().length > 0 ) );
	input.addEventListener( 'blur', () => show( false ) );
	// A click on an option keeps the focus in the field: the list stays
	// until the link opens.
	list.addEventListener( 'mousedown', ( event ) => event.preventDefault() );
} )();

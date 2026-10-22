// The editor of a label (#label-editor): a label that is not in the page
// language gets its translation in a dialog. Progressive enhancement only:
// without this script, the action of a label is a link to
// Special:SetLabel on Wikidata. The machine translation of Wikimedia
// (MinT, by cxserver) fills the field first, and the reader corrects it.
// "Save" mimics a published edit: every place of the page with the label
// shows the new one. Nothing is sent.
// One function scope: the page scripts share one script element.
( () => {
	const dialog = document.getElementById( 'label-editor' );
	if ( !dialog ) {
		return;
	}
	const MT = 'https://cxserver.wikimedia.org/v2/translate';
	const root = document.documentElement;
	const page = root.lang;
	const input = dialog.querySelector( 'input' );
	const status = dialog.querySelector( '.status' );
	const sourceText = dialog.querySelector( '.source .text' );
	const sourceLang = dialog.querySelector( '.source .source-lang' );
	let names = null;
	try {
		names = new Intl.DisplayNames( root.dataset.locale || page, { type: 'language', fallback: 'none' } );
	} catch ( e ) {
		// The code of the language stays.
	}
	// The labels saved on this page, by the address of their action: the
	// source label, its language, and the saved text.
	const saved = new Map();
	// The address of the action in the dialog, and its request.
	let current = null;
	let controller = null;

	// The label of the group of an action: in a link or bare.
	const labelOf = ( action ) => action.parentElement.querySelector( '.fb, .localized' );

	// The name of a language in the page locale; else its code.
	const nameOf = ( code ) => {
		try {
			return ( names && names.of( code ) ) || code;
		} catch ( e ) {
			return code;
		}
	};

	// The machine translation of a text: the text of the answer only. The
	// answer is parsed in a template, which runs and loads nothing.
	const translate = async ( text, from, signal ) => {
		const p = document.createElement( 'p' );
		p.textContent = text;
		const response = await fetch( `${ MT }/${ from }/${ page }/MinT`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify( { html: `<section>${ p.outerHTML }</section>` } ),
			signal
		} );
		if ( !response.ok ) {
			throw new Error( response.status );
		}
		const { contents } = await response.json();
		const template = document.createElement( 'template' );
		template.innerHTML = contents;
		const result = template.content.textContent.trim();
		if ( !result ) {
			throw new Error( 'empty' );
		}
		return result;
	};

	const open = async ( action ) => {
		current = action.getAttribute( 'href' );
		const label = labelOf( action );
		const entry = saved.get( current ) ||
			{ text: label.textContent, lang: label.lang, value: '' };
		sourceText.textContent = entry.text;
		sourceText.lang = entry.lang;
		sourceLang.textContent = nameOf( entry.lang );
		input.value = entry.value;
		status.textContent = '';
		dialog.showModal();
		input.focus();
		if ( entry.value ) {
			return;
		}
		// A new label: its machine translation, unless the reader has
		// started to write.
		controller = new AbortController();
		status.textContent = status.dataset.translating;
		try {
			const result = await translate( entry.text, entry.lang, controller.signal );
			if ( !input.value ) {
				input.value = result;
				input.select();
			}
			status.textContent = '';
		} catch ( e ) {
			if ( e.name !== 'AbortError' ) {
				status.textContent = status.dataset.failed;
			}
		}
	};

	// Every place of the page with the label shows the saved text, in the
	// page language, marked as saved on this page only.
	const save = () => {
		const value = input.value.trim();
		if ( !current || !value ) {
			return;
		}
		const actions = [ ...document.querySelectorAll( '.fb-group > a.add-label' ) ]
			.filter( ( a ) => a.getAttribute( 'href' ) === current );
		const first = labelOf( actions[ 0 ] );
		const entry = saved.get( current ) || { text: first.textContent, lang: first.lang };
		saved.set( current, { ...entry, value } );
		for ( const action of actions ) {
			const label = labelOf( action );
			label.textContent = value;
			label.lang = page;
			label.dir = root.dir;
			label.title = dialog.dataset.saved;
			label.classList.replace( 'fb', 'localized' );
		}
	};

	dialog.addEventListener( 'close', () => {
		controller?.abort();
		current = null;
		status.textContent = '';
	} );
	dialog.querySelector( '.close' ).addEventListener( 'click', () => dialog.close() );
	dialog.querySelector( '.save' ).addEventListener( 'click', () => {
		save();
		dialog.close();
	} );
	input.addEventListener( 'keydown', ( event ) => {
		if ( event.key === 'Enter' ) {
			save();
			dialog.close();
		}
	} );

	document.addEventListener( 'click', ( event ) => {
		const action = event.target.closest( '.fb-group > a.add-label' );
		if ( action ) {
			event.preventDefault();
			open( action );
			return;
		}
		// A click on a label that is not a link shows its action; a click
		// elsewhere hides it again.
		const group = event.target.closest( '.fb-group' );
		for ( const shown of document.querySelectorAll( '.fb-group.shown' ) ) {
			if ( shown !== group ) {
				shown.classList.remove( 'shown' );
			}
		}
		if ( group && !event.target.closest( 'a' ) ) {
			group.classList.add( 'shown' );
		}
	} );
} )();

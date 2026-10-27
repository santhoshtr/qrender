// Progressive enhancement only: a long text of the fallback article (the
// lead and each top-level section, a qr-section) shows its start, and
// "Read more in English" under it shows the whole text. The whole text is
// in the page: the script adds the class that clamps it (CSS), and the
// words of both states are in the button. Without this script, the page
// shows the whole text. A text that is at most 1.5 × the clamp is not
// clamped: its button stays hidden.
( () => {
	const LIMIT = 1.5;
	const texts = [ ...document.querySelectorAll( 'qr-section' ) ].flatMap( ( section ) => {
		const body = section.querySelector( ':scope > .body' );
		const more = section.querySelector( ':scope > .actions > .more' );
		return body && more ? [ { section, body, more } ] : [];
	} );

	// Clamp the text when it is long enough; else show it whole. An open
	// text stays open.
	const measure = ( { section, body, more } ) => {
		if ( section.classList.contains( 'open' ) ) {
			return;
		}
		section.classList.add( 'clamped' );
		const long = body.scrollHeight > LIMIT * body.clientHeight;
		section.classList.toggle( 'clamped', long );
		more.hidden = !long;
	};

	const toggle = ( { section, more }, open ) => {
		section.classList.toggle( 'open', open );
		more.setAttribute( 'aria-expanded', String( open ) );
	};

	for ( const text of texts ) {
		measure( text );
		text.more.addEventListener( 'click', () => {
			const open = !text.section.classList.contains( 'open' );
			toggle( text, open );
			// Closed while the reader is past its start: go to its start.
			if ( !open && text.section.getBoundingClientRect().top < 0 ) {
				text.section.scrollIntoView();
			}
		} );
		// A link in the hidden part, by Tab: show the whole text.
		text.body.addEventListener( 'focusin', () => {
			if ( text.section.classList.contains( 'clamped' ) ) {
				toggle( text, true );
			}
		} );
	}

	// The width changes the height of a text: measure again.
	let frame = 0;
	window.addEventListener( 'resize', () => {
		cancelAnimationFrame( frame );
		frame = requestAnimationFrame( () => texts.forEach( measure ) );
	} );
} )();

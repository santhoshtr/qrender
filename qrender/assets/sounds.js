// Progressive enhancement only: the recordings of one property (the
// pronunciations of the name) become one player with a choice of
// language. Without this script every recording keeps its own player.
for ( const block of document.querySelectorAll( '.sounds' ) ) {
	const choice = block.querySelector( '.sound-choice' );
	const sounds = [ ...block.querySelectorAll( '.sound' ) ];
	const show = ( chosen ) => {
		sounds.forEach( ( sound, i ) => {
			sound.hidden = i !== chosen;
			if ( i !== chosen ) {
				sound.querySelector( 'audio' ).pause();
			}
		} );
	};
	choice.hidden = false;
	show( 0 );
	choice.addEventListener( 'change', () => show( Number( choice.value ) ) );
}

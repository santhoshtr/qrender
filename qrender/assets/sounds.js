// The qr-sounds element: the recordings of one property (the
// pronunciations of the name) as one player with a choice of language.
// Progressive enhancement only: an element that is not defined shows its
// children, so without this script every recording keeps its own player.
if ( 'customElements' in window ) {
	customElements.define( 'qr-sounds', class extends HTMLElement {
		connectedCallback() {
			const choice = this.querySelector( '.sound-choice' );
			const sounds = [ ...this.querySelectorAll( '.sound' ) ];
			if ( !choice || sounds.length < 2 ) {
				return;
			}
			const show = ( chosen ) => {
				sounds.forEach( ( sound, i ) => {
					sound.hidden = i !== chosen;
					if ( i !== chosen ) {
						sound.querySelector( 'audio' ).pause();
					}
				} );
			};
			choice.hidden = false;
			show( Number( choice.value ) || 0 );
			choice.addEventListener( 'change', () => show( Number( choice.value ) ) );
		}
	} );
}

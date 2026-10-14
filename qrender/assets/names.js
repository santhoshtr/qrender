// The qr-names element: the names of the item in many languages, as one
// choice of language and the group of that language. Progressive
// enhancement only: the server renders every group, the page language
// first; this script shows the choice and one group, and decides nothing.
if ( 'customElements' in window ) {
	customElements.define( 'qr-names', class extends HTMLElement {
		connectedCallback() {
			const choice = this.querySelector( '.name-choice' );
			const groups = [ ...this.querySelectorAll( '.name-group' ) ];
			if ( !choice || groups.length < 2 ) {
				return;
			}
			const show = ( lang ) => groups.forEach( ( group ) => {
				group.hidden = group.dataset.lang !== lang;
			} );
			choice.hidden = false;
			show( choice.value );
			choice.addEventListener( 'change', () => show( choice.value ) );
		}
	} );
}

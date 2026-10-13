// Progressive enhancement only: opens and closes dialogs in browsers
// without the invoker commands API. The content does not need it.
if ( !( 'commandForElement' in HTMLButtonElement.prototype ) ) {
	document.addEventListener( 'click', ( event ) => {
		const button = event.target.closest( 'button[commandfor]' );
		if ( !button ) {
			return;
		}
		const target = document.getElementById( button.getAttribute( 'commandfor' ) );
		if ( !target ) {
			return;
		}
		if ( button.getAttribute( 'command' ) === 'close' ) {
			target.close();
		} else {
			target.showModal();
		}
	} );
}

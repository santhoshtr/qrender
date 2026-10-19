// Progressive enhancement only: a video plays in its dialog. It starts when
// the dialog opens (the reader asked for it with the play button), and it
// stops when the dialog closes. Without this script, the reader starts it
// with its controls, and it plays on after the dialog closes.
// The toggle and close events of a dialog do not bubble: they are heard in
// the capture phase.
document.addEventListener( 'toggle', ( event ) => {
	if ( event.newState === 'open' && event.target.matches( 'dialog.lightbox' ) ) {
		// A browser can refuse to play; the controls are there.
		event.target.querySelector( 'video' )?.play().catch( () => {} );
	}
}, true );

document.addEventListener( 'close', ( event ) => {
	event.target.querySelector( 'video' )?.pause();
}, true );

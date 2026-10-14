// Progressive enhancement only: a play link (a.play) of an audio file
// plays the file in the page; a second click stops it. One sound plays at
// a time. Without this script the link opens the file, and the browser
// plays it.
( () => {
	let sound, link;
	const stop = () => {
		sound.pause();
		link.classList.remove( 'playing' );
		sound = link = null;
	};
	document.addEventListener( 'click', ( event ) => {
		const a = event.target.closest( 'a.play' );
		if ( !a ) {
			return;
		}
		event.preventDefault();
		const again = link === a;
		if ( sound ) {
			stop();
		}
		if ( again ) {
			return;
		}
		sound = new Audio( a.href );
		link = a;
		a.classList.add( 'playing' );
		sound.addEventListener( 'ended', () => link === a && stop() );
		sound.play().catch( () => {
			window.location.href = a.href;
		} );
	} );
} )();

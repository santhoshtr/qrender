// Progressive enhancement only: a link in the contents drawer (a narrow
// page) closes the drawer, so the reader sees the spread it goes to. A
// popover does not close when a link in it is clicked. Without this
// script, a tap outside the drawer or Escape closes it.
( () => {
	document.getElementById( 'toc' )?.addEventListener( 'click', ( event ) => {
		const toc = event.currentTarget;
		if ( event.target.closest( 'a' ) && toc.matches( ':popover-open' ) ) {
			toc.hidePopover();
		}
	} );
} )();

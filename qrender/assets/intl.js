// Progressive enhancement only: numbers and dates in the formats of the
// page locale, by the browser's Intl. The server writes a neutral text
// (digits without grouping, ISO dates); without this script it stays,
// correct but not localized.
( () => {
	const locale = document.documentElement.dataset.locale || document.documentElement.lang;
	let number, percent;
	try {
		number = new Intl.NumberFormat( locale, { maximumFractionDigits: 20 } );
		percent = new Intl.NumberFormat( locale, { style: 'percent', maximumSignificantDigits: 2 } );
	} catch ( e ) {
		return;
	}
	for ( const el of document.querySelectorAll( 'data[data-f]' ) ) {
		const value = Number( el.value );
		if ( Number.isNaN( value ) ) {
			continue;
		}
		if ( el.dataset.f === 'number' ) {
			el.textContent = number.format( value );
		} else if ( el.dataset.f === 'percent' ) {
			el.textContent = percent.format( value );
		}
		// "scientific" keeps its neutral text: m × 10ⁿ.
	}
	// Wikidata precision: 9 year, 10 month, 11 day.
	const options = {
		9: { year: 'numeric' },
		10: { year: 'numeric', month: 'long' },
		11: { year: 'numeric', month: 'long', day: 'numeric' }
	};
	for ( const el of document.querySelectorAll( 'time[datetime][data-p]' ) ) {
		const [ year, month = 1, day = 1 ] = el.dateTime.split( '-' ).map( Number );
		const date = new Date( Date.UTC( 2000, month - 1, day ) );
		date.setUTCFullYear( year ); // Date.UTC reads years below 100 as 19xx
		try {
			el.textContent = new Intl.DateTimeFormat( locale,
				{ ...options[ Math.min( Number( el.dataset.p ), 11 ) ], timeZone: 'UTC' } ).format( date );
		} catch ( e ) {
			// The neutral text stays.
		}
	}
} )();

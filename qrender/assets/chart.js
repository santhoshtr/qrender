// The qr-chart element: the points of a series (the columns of its table)
// as a line chart, a line for each column, drawn by ECharts. Progressive enhancement only: the server
// renders the table, and without this script or without ECharts the table
// stays. The script reads the rows and decides nothing. ECharts loads
// once, when the first chart comes near the viewport, so it does not block
// the render.
if ( 'customElements' in window && 'IntersectionObserver' in window ) {
	const root = document.documentElement;
	// The design base is on a page that the server sends (not in a file).
	const src = root.dataset.design && `${ root.dataset.design }/echarts.js`;
	const locale = root.dataset.locale || root.lang;
	let loading;
	const echarts = () => loading ??= new Promise( ( resolve, reject ) => {
		const script = document.createElement( 'script' );
		script.src = src;
		script.async = true;
		script.onload = () => resolve( window.echarts );
		script.onerror = reject;
		document.head.append( script );
	} );
	const near = new IntersectionObserver( ( entries ) => {
		for ( const entry of entries ) {
			if ( entry.isIntersecting ) {
				near.unobserve( entry.target );
				entry.target.draw();
			}
		}
	}, { rootMargin: '200px' } );

	customElements.define( 'qr-chart', class extends HTMLElement {
		connectedCallback() {
			if ( src ) {
				near.observe( this );
			}
		}

		disconnectedCallback() {
			near.unobserve( this );
		}

		async draw() {
			const box = this.querySelector( '.chart' );
			const rows = [ ...this.querySelectorAll( 'tbody tr[data-year]' ) ];
			const names = [ ...this.querySelectorAll( 'thead th' ) ].slice( 1 )
				.map( ( th ) => th.textContent.trim() );
			// A line goes through one value of each year; the other values of
			// a year (other sources) are marks off the line.
			const lines = names.map( ( name, i ) => {
				const cells = rows.map( ( row ) => [ Number( row.dataset.year ), row.cells[ i + 1 ] ] );
				return {
					name,
					points: cells.filter( ( [ , td ] ) => td && 'amount' in td.dataset )
						.map( ( [ y, td ] ) => [ y, Number( td.dataset.amount ) ] ),
					others: cells.flatMap( ( [ y, td ] ) => td ?
						[ ...td.querySelectorAll( '[data-other]' ) ].map( ( o ) => [ y, Number( o.dataset.other ) ] ) :
						[] )
				};
			} );
			const otherNote = this.querySelector( '[data-other] .note' )?.textContent ?? '';
			if ( !box || !lines.some( ( l ) => l.points.length ) ) {
				return;
			}
			let lib;
			try {
				lib = await echarts();
			} catch ( e ) {
				return; // The table stays.
			}
			let year, exact, short;
			try {
				year = new Intl.NumberFormat( locale, { useGrouping: false } );
				exact = new Intl.NumberFormat( locale, { maximumFractionDigits: 20 } );
				short = new Intl.NumberFormat( locale, { notation: 'compact', maximumSignificantDigits: 4 } );
			} catch ( e ) {
				return;
			}
			const style = getComputedStyle( this );
			const token = ( name ) => style.getPropertyValue( name ).trim();
			const subtle = token( '--color-subtle' );
			const rtl = getComputedStyle( box ).direction === 'rtl';
			const single = lines.length === 1;
			// One line: the colour of the dimension. Lines of a split: the
			// colours of the parts, as in their key.
			const colour = ( i ) => token( single ? '--ink' : `--part-${ i + 1 }` );
			const years = [ ...new Set( lines.flatMap( ( l ) => l.points.map( ( [ y ] ) => y ) ) ) ]
				.sort( ( a, b ) => a - b );
			const few = years.length <= 6;
			const html = lib.format.encodeHTML;
			const pad = ( min, max ) => Math.max( 0.5, ( max - min ) * 0.1 );

			const plot = document.createElement( 'div' );
			plot.className = 'plot';
			plot.setAttribute( 'aria-hidden', 'true' );
			// The options turn the chart for a right-to-left page; the SVG
			// text must not turn again by the page direction.
			plot.dir = 'ltr';
			box.append( plot );
			const chart = lib.init( plot, null, { renderer: 'svg' } );
			chart.setOption( {
				animation: false,
				textStyle: { fontFamily: style.fontFamily },
				grid: { left: 8, right: 16, top: 24, bottom: 8, containLabel: true },
				tooltip: {
					trigger: 'axis',
					backgroundColor: token( '--background-color-base' ),
					borderColor: token( '--border-color-base' ),
					textStyle: { color: token( '--color-base' ) },
					// Each series gives its nearest point: only the values of
					// the year of a line's point. Series 2i is line i, 2i + 1
					// its other values.
					formatter: ( params ) => {
						const at = ( params.find( ( p ) => p.seriesIndex % 2 === 0 ) ?? params[ 0 ] ).value[ 0 ];
						const value = ( p ) => exact.format( p.value[ 1 ] ) +
							( p.seriesIndex % 2 ? ` (${ html( otherNote ) })` : '' );
						const of = params.filter( ( p ) => p.value[ 0 ] === at );
						if ( single ) {
							return `${ year.format( at ) }: ${ of.map( value ).join( ' · ' ) }`;
						}
						return [ year.format( at ), ...of.map( ( p ) =>
							`${ p.marker }${ html( p.seriesName ) }: ${ value( p ) }` ) ].join( '<br>' );
					}
				},
				xAxis: {
					type: 'value',
					inverse: rtl,
					// Room at each end for the end points and their numbers.
					min: ( { min, max } ) => min - pad( min, max ),
					max: ( { min, max } ) => max + pad( min, max ),
					minInterval: 1,
					splitLine: { show: false },
					axisLine: { lineStyle: { color: token( '--border-color-base' ) } },
					// A few points: each year of the data has its label.
					axisTick: few ? { customValues: years } : {},
					axisLabel: {
						color: subtle,
						hideOverlap: true,
						...( few ? { customValues: years } : {} ),
						formatter: ( v ) => Number.isInteger( v ) ? year.format( v ) : ''
					}
				},
				yAxis: {
					type: 'value',
					position: rtl ? 'right' : 'left',
					scale: true,
					splitLine: { lineStyle: { color: token( '--border-color-subtle' ) } },
					axisLabel: { color: subtle, formatter: ( v ) => short.format( v ) }
				},
				series: lines.flatMap( ( l, i ) => [ {
					type: 'line',
					name: l.name,
					data: l.points,
					color: colour( i ),
					symbolSize: 6,
					showSymbol: l.points.length <= 24,
					label: {
						// The numbers on the points of one short line; with
						// lines, they would cover each other.
						show: single && few,
						color: subtle,
						formatter: ( p ) => short.format( p.value[ 1 ] )
					}
				}, {
					type: 'scatter',
					name: l.name,
					data: l.others,
					color: colour( i ),
					symbol: 'emptyCircle',
					symbolSize: 7
				} ] )
			} );
			this.classList.add( 'drawn' );
			new ResizeObserver( () => chart.resize() ).observe( box );
		}
	} );
}

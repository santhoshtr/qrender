// The qr-chart element: the points of a series (the rows of its table) as
// a line chart, drawn by ECharts. Progressive enhancement only: the server
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
			const points = [ ...this.querySelectorAll( 'tr[data-year]' ) ]
				.map( ( row ) => [ Number( row.dataset.year ), Number( row.dataset.amount ) ] );
			if ( !box || !points.length ) {
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
			const few = points.length <= 6;
			const years = points.map( ( [ y ] ) => y );
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
					formatter: ( [ p ] ) => `${ year.format( p.value[ 0 ] ) }: ${ exact.format( p.value[ 1 ] ) }`
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
				series: [ {
					type: 'line',
					data: points,
					color: token( '--ink' ),
					symbolSize: 6,
					showSymbol: points.length <= 24,
					label: {
						show: few,
						color: subtle,
						formatter: ( p ) => short.format( p.value[ 1 ] )
					}
				} ]
			} );
			this.classList.add( 'drawn' );
			new ResizeObserver( () => chart.resize() ).observe( box );
		}
	} );
}

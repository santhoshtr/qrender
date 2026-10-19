// The qr-model element: a 3D model (a binary STL file) that the reader turns
// (drag) and zooms (wheel), drawn by WebGL2 with no library. Progressive
// enhancement only: the element holds the picture that Commons renders from
// the file, and without WebGL2 or this script the picture stays. The file is
// large (tens of MB), so it loads only when the dialog of the element opens,
// and a close stops the load. A model that is loaded stays.
// One function scope: the page scripts share one script element.
( () => {
	if ( !( 'customElements' in window ) ) {
		return;
	}

	const percent = new Intl.NumberFormat( document.documentElement.lang, { style: 'percent' } );

	customElements.define( 'qr-model', class extends HTMLElement {
		connectedCallback() {
			this.dialog = this.closest( 'dialog' );
			this.dialog?.addEventListener( 'toggle', ( event ) => {
				if ( event.newState === 'open' ) {
					this.open();
				} else {
					this.controller?.abort();
				}
			} );
		}

		async open() {
			if ( this.controller || this.done ) {
				return;
			}
			const canvas = document.createElement( 'canvas' );
			const gl = canvas.getContext( 'webgl2' );
			if ( !gl ) {
				return;
			}
			const status = this.querySelector( '.status' );
			const total = Number( this.dataset.bytes );
			const progress = ( received ) => {
				status.textContent = this.dataset.loading.replace( '$1', total ? percent.format( received / total ) : '' );
			};
			this.controller = new AbortController();
			progress( 0 );
			try {
				const buffer = await load( this.getAttribute( 'src' ), this.controller.signal, progress );
				const mesh = parse( buffer );
				status.textContent = mesh ? '' : this.dataset.unsupported;
				if ( mesh ) {
					draw( this, canvas, gl, mesh );
				}
				this.done = true;
			} catch {
				// Stopped or failed: the picture stays, and the next open tries again.
				status.textContent = '';
			} finally {
				this.controller = null;
			}
		}
	} );

	async function load( url, signal, progress ) {
		const response = await fetch( url, { signal } );
		if ( !response.ok ) {
			throw new Error( response.status );
		}
		const reader = response.body.getReader();
		const chunks = [];
		let received = 0;
		for ( ;; ) {
			const { done, value } = await reader.read();
			if ( done ) {
				break;
			}
			chunks.push( value );
			received += value.length;
			progress( received );
		}
		const buffer = new Uint8Array( received );
		let offset = 0;
		for ( const chunk of chunks ) {
			buffer.set( chunk, offset );
			offset += chunk.length;
		}
		return buffer.buffer;
	}

	// Binary STL: an 80-byte header, a triangle count, then 50 bytes a triangle.
	// A file of another size is not binary STL (an ASCII STL): null. Normals
	// come from the vertices; the stored ones are often empty.
	function parse( buffer ) {
		const view = new DataView( buffer );
		const count = buffer.byteLength >= 84 ? view.getUint32( 80, true ) : 0;
		if ( !count || buffer.byteLength !== 84 + count * 50 ) {
			return null;
		}
		const position = new Float32Array( count * 9 );
		const normal = new Float32Array( count * 9 );
		const min = [ Infinity, Infinity, Infinity ], max = [ -Infinity, -Infinity, -Infinity ];
		for ( let i = 0; i < count; i++ ) {
			const base = 84 + i * 50 + 12;
			for ( let j = 0; j < 9; j++ ) {
				const v = view.getFloat32( base + j * 4, true );
				position[ i * 9 + j ] = v;
				min[ j % 3 ] = Math.min( min[ j % 3 ], v );
				max[ j % 3 ] = Math.max( max[ j % 3 ], v );
			}
			const p = position.subarray( i * 9, i * 9 + 9 );
			const ux = p[ 3 ] - p[ 0 ], uy = p[ 4 ] - p[ 1 ], uz = p[ 5 ] - p[ 2 ];
			const vx = p[ 6 ] - p[ 0 ], vy = p[ 7 ] - p[ 1 ], vz = p[ 8 ] - p[ 2 ];
			let nx = uy * vz - uz * vy, ny = uz * vx - ux * vz, nz = ux * vy - uy * vx;
			const length = Math.hypot( nx, ny, nz ) || 1;
			nx /= length; ny /= length; nz /= length;
			for ( let k = 0; k < 3; k++ ) {
				normal[ i * 9 + k * 3 ] = nx; normal[ i * 9 + k * 3 + 1 ] = ny; normal[ i * 9 + k * 3 + 2 ] = nz;
			}
		}
		const centre = [ 0, 1, 2 ].map( ( k ) => ( min[ k ] + max[ k ] ) / 2 );
		const size = Math.max( max[ 0 ] - min[ 0 ], max[ 1 ] - min[ 1 ], max[ 2 ] - min[ 2 ] );
		return { position, normal, count: count * 3, centre, size };
	}

	// The mesh, grey and lit from the upper left, on a transparent canvas (the
	// stage of the dialog gives the background). It turns slowly until the
	// reader drags it, and not at all with reduced motion. It is drawn only
	// while its dialog is open.
	function draw( element, canvas, gl, mesh ) {
		const program = gl.createProgram();
		[ [ gl.VERTEX_SHADER, `#version 300 es
			in vec3 position; in vec3 normal;
			uniform mat4 model; uniform mat4 projection;
			out vec3 vNormal;
			void main() {
				vNormal = mat3( model ) * normal;
				gl_Position = projection * model * vec4( position, 1.0 );
			}` ], [ gl.FRAGMENT_SHADER, `#version 300 es
			precision mediump float;
			in vec3 vNormal; out vec4 colour;
			void main() {
				float light = max( dot( normalize( vNormal ), normalize( vec3( -0.5, 0.6, 0.8 ) ) ), 0.0 );
				colour = vec4( vec3( 0.12 + 0.82 * light ), 1.0 );
			}` ] ].forEach( ( [ type, source ] ) => {
			const shader = gl.createShader( type );
			gl.shaderSource( shader, source );
			gl.compileShader( shader );
			gl.attachShader( program, shader );
		} );
		gl.linkProgram( program );
		gl.useProgram( program );
		[ [ 'position', mesh.position ], [ 'normal', mesh.normal ] ].forEach( ( [ name, data ] ) => {
			gl.bindBuffer( gl.ARRAY_BUFFER, gl.createBuffer() );
			gl.bufferData( gl.ARRAY_BUFFER, data, gl.STATIC_DRAW );
			const location = gl.getAttribLocation( program, name );
			gl.enableVertexAttribArray( location );
			gl.vertexAttribPointer( location, 3, gl.FLOAT, false, 0, 0 );
		} );
		gl.enable( gl.DEPTH_TEST );
		gl.enable( gl.CULL_FACE );
		gl.clearColor( 0, 0, 0, 0 );
		const modelAt = gl.getUniformLocation( program, 'model' );
		const projectionAt = gl.getUniformLocation( program, 'projection' );
		const count = mesh.count, s = 2 / mesh.size, [ cx, cy, cz ] = mesh.centre;

		const still = matchMedia( '(prefers-reduced-motion: reduce)' ).matches;
		let yaw = 0, pitch = 0.3, distance = 3.4, dragging = null, spinning = !still, pending = 0;

		function frame() {
			pending = 0;
			if ( !element.dialog.open ) {
				return;
			}
			const w = canvas.clientWidth * devicePixelRatio, h = canvas.clientHeight * devicePixelRatio;
			if ( canvas.width !== w || canvas.height !== h ) {
				canvas.width = w;
				canvas.height = h;
			}
			gl.viewport( 0, 0, w, h );
			gl.clear( gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT );
			if ( spinning ) {
				yaw += 0.003;
			}
			const cyw = Math.cos( yaw ), syw = Math.sin( yaw ), cp = Math.cos( pitch ), sp = Math.sin( pitch );
			// model = translate(0,0,-distance) * rotateX(pitch) * rotateY(yaw) * scale * translate(-centre)
			const r = [ cyw, sp * syw, -cp * syw, 0, 0, cp, sp, 0, syw, -sp * cyw, cp * cyw, 0 ];
			const model = new Float32Array( 16 );
			for ( let c = 0; c < 3; c++ ) {
				for ( let k = 0; k < 3; k++ ) {
					model[ c * 4 + k ] = r[ c * 4 + k ] * s;
				}
			}
			for ( let k = 0; k < 3; k++ ) {
				model[ 12 + k ] = -( r[ k ] * cx + r[ 4 + k ] * cy + r[ 8 + k ] * cz ) * s;
			}
			model[ 14 ] -= distance;
			model[ 15 ] = 1;
			const f = 1 / Math.tan( 0.4 ), near = 0.1, far = 20, aspect = w / h;
			const projection = new Float32Array( [ f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, ( far + near ) / ( near - far ), -1, 0, 0, 2 * far * near / ( near - far ), 0 ] );
			gl.uniformMatrix4fv( modelAt, false, model );
			gl.uniformMatrix4fv( projectionAt, false, projection );
			gl.drawArrays( gl.TRIANGLES, 0, count );
			run();
		}
		const run = () => {
			pending ||= requestAnimationFrame( frame );
		};

		canvas.addEventListener( 'pointerdown', ( event ) => {
			dragging = [ event.clientX, event.clientY ];
			spinning = false;
			canvas.setPointerCapture( event.pointerId );
		} );
		canvas.addEventListener( 'pointermove', ( event ) => {
			if ( !dragging ) {
				return;
			}
			yaw += ( event.clientX - dragging[ 0 ] ) * 0.01;
			pitch = Math.max( -1.5, Math.min( 1.5, pitch + ( event.clientY - dragging[ 1 ] ) * 0.01 ) );
			dragging = [ event.clientX, event.clientY ];
		} );
		canvas.addEventListener( 'pointerup', () => {
			dragging = null;
		} );
		canvas.addEventListener( 'wheel', ( event ) => {
			event.preventDefault();
			distance = Math.max( 1.2, Math.min( 6, distance * ( 1 + event.deltaY * 0.001 ) ) );
		}, { passive: false } );
		element.dialog.addEventListener( 'toggle', ( event ) => {
			if ( event.newState === 'open' ) {
				run();
			}
		} );

		canvas.setAttribute( 'role', 'img' );
		canvas.setAttribute( 'aria-label', element.dataset.label );
		element.querySelector( 'img' ).replaceWith( canvas );
		run();
	}
} )();

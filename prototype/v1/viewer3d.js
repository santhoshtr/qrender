// Progressive enhancement only. Without it the figure shows the picture
// that Commons renders from the same file. The model loads when the reader
// asks for it, because it is large.
document.querySelectorAll( '.model3d' ).forEach( function ( figure ) {
	const button = figure.querySelector( '.model3d-load' );
	const canvas = document.createElement( 'canvas' );
	const gl = canvas.getContext( 'webgl2' );
	if ( !gl ) { return; }
	button.hidden = false;
	button.addEventListener( 'click', function () {
		button.disabled = true;
		load( figure.dataset.src, button ).then( function ( buffer ) {
			show( figure, canvas, gl, parse( buffer ) );
		} ).catch( function () {
			button.disabled = false;
		} );
	} );
} );

async function load( url, button ) {
	const response = await fetch( url );
	const total = Number( response.headers.get( 'Content-Length' ) );
	const reader = response.body.getReader();
	const chunks = [];
	let received = 0;
	for ( ;; ) {
		const { done, value } = await reader.read();
		if ( done ) { break; }
		chunks.push( value );
		received += value.length;
		button.textContent = total ? Math.round( received / total * 100 ) + ' %' :
			( received / 1e6 ).toFixed( 1 ) + ' MB';
	}
	const buffer = new Uint8Array( received );
	let offset = 0;
	for ( const chunk of chunks ) { buffer.set( chunk, offset ); offset += chunk.length; }
	return buffer.buffer;
}

// Binary STL: an 80-byte header, a triangle count, then 50 bytes a triangle.
// Normals come from the vertices; the stored ones are often empty.
function parse( buffer ) {
	const view = new DataView( buffer );
	const count = view.getUint32( 80, true );
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

function show( figure, canvas, gl, mesh ) {
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
		}` ] ].forEach( function ( [ type, source ] ) {
		const shader = gl.createShader( type );
		gl.shaderSource( shader, source );
		gl.compileShader( shader );
		gl.attachShader( program, shader );
	} );
	gl.linkProgram( program );
	gl.useProgram( program );
	[ [ 'position', mesh.position ], [ 'normal', mesh.normal ] ].forEach( function ( [ name, data ] ) {
		gl.bindBuffer( gl.ARRAY_BUFFER, gl.createBuffer() );
		gl.bufferData( gl.ARRAY_BUFFER, data, gl.STATIC_DRAW );
		const location = gl.getAttribLocation( program, name );
		gl.enableVertexAttribArray( location );
		gl.vertexAttribPointer( location, 3, gl.FLOAT, false, 0, 0 );
	} );
	gl.enable( gl.DEPTH_TEST );
	gl.enable( gl.CULL_FACE );
	mesh.position = mesh.normal = null;

	const still = matchMedia( '(prefers-reduced-motion: reduce)' ).matches;
	let yaw = 0, pitch = 0.3, distance = 3.4, dragging = null, spinning = !still;
	const s = 2 / mesh.size, [ cx, cy, cz ] = mesh.centre;

	function frame() {
		const w = canvas.clientWidth * devicePixelRatio, h = canvas.clientHeight * devicePixelRatio;
		if ( canvas.width !== w || canvas.height !== h ) { canvas.width = w; canvas.height = h; }
		gl.viewport( 0, 0, w, h );
		gl.clearColor( 0, 0, 0, 1 );
		gl.clear( gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT );
		if ( spinning ) { yaw += 0.003; }
		const cyw = Math.cos( yaw ), syw = Math.sin( yaw ), cp = Math.cos( pitch ), sp = Math.sin( pitch );
		// model = translate(0,0,-distance) * rotateX(pitch) * rotateY(yaw) * scale * translate(-centre)
		const r = [ cyw, sp * syw, -cp * syw, 0, 0, cp, sp, 0, syw, -sp * cyw, cp * cyw, 0 ];
		const model = new Float32Array( 16 );
		for ( let c = 0; c < 3; c++ ) { for ( let k = 0; k < 3; k++ ) { model[ c * 4 + k ] = r[ c * 4 + k ] * s; } }
		for ( let k = 0; k < 3; k++ ) { model[ 12 + k ] = -( r[ k ] * cx + r[ 4 + k ] * cy + r[ 8 + k ] * cz ) * s; }
		model[ 14 ] -= distance;
		model[ 15 ] = 1;
		const f = 1 / Math.tan( 0.4 ), near = 0.1, far = 20, aspect = w / h;
		const projection = new Float32Array( [ f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, ( far + near ) / ( near - far ), -1, 0, 0, 2 * far * near / ( near - far ), 0 ] );
		gl.uniformMatrix4fv( gl.getUniformLocation( program, 'model' ), false, model );
		gl.uniformMatrix4fv( gl.getUniformLocation( program, 'projection' ), false, projection );
		gl.drawArrays( gl.TRIANGLES, 0, mesh.count );
		requestAnimationFrame( frame );
	}

	canvas.addEventListener( 'pointerdown', function ( event ) {
		dragging = [ event.clientX, event.clientY ];
		spinning = false;
		canvas.setPointerCapture( event.pointerId );
	} );
	canvas.addEventListener( 'pointermove', function ( event ) {
		if ( !dragging ) { return; }
		yaw += ( event.clientX - dragging[ 0 ] ) * 0.01;
		pitch = Math.max( -1.5, Math.min( 1.5, pitch + ( event.clientY - dragging[ 1 ] ) * 0.01 ) );
		dragging = [ event.clientX, event.clientY ];
	} );
	canvas.addEventListener( 'pointerup', function () { dragging = null; } );
	canvas.addEventListener( 'wheel', function ( event ) {
		event.preventDefault();
		distance = Math.max( 1.2, Math.min( 6, distance * ( 1 + event.deltaY * 0.001 ) ) );
	}, { passive: false } );

	canvas.setAttribute( 'aria-label', figure.querySelector( 'img' ).alt );
	figure.querySelector( '.model3d-stage' ).replaceChildren( canvas );
	figure.querySelector( '.model3d-load' ).hidden = true;
	requestAnimationFrame( frame );
}

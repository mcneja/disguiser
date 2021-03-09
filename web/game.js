window.onload = loadResourcesThenRun;

// Buffer for accumulating geometry to be sent for rendering
// Position: four vertices per quad, four components per position (x, y, s, t)
// Colors: four colors per quad, four components (RGBA) per color

const maxQuads = 4096;
const vertexPositions = new Float32Array(maxQuads * 16);
const vertexColors = new Uint32Array(maxQuads * 4);
let numQuads = 0;

// Projection matrix memory

const projectionMatrix = new Float32Array(16);

// Functions

const loadImage = src =>
	new Promise((resolve, reject) => {
		const img = new Image();
		img.onload = () => resolve(img);
		img.onerror = reject;
		img.src = src;
	});

function loadResourcesThenRun() {
	Promise.all([
		loadImage('tiles.png'),
		loadImage('font.png'),
		fetch('game.wasm'),
	]).then(([tileImage, fontImage, wasm]) => {
		main([tileImage, fontImage], wasm);
	});
}

function main(textureImages, wasm) {

	// The projection matrix mostly stays as zeroes

	projectionMatrix.fill(0);
	projectionMatrix[10] = 1;
	projectionMatrix[12] = -1;
	projectionMatrix[13] = -1;
	projectionMatrix[15] = 1;

	// Initialize all WebGL resources

	const canvas = document.querySelector("#canvas");
	const gl = canvas.getContext("webgl", { alpha: false, antialias: false, depth: false });

	if (gl == null) {
		alert("Unable to initialize WebGL. Your browser or machine may not support it.");
		return;
	}

	// Set up various WebGL state that won't change for the duration of the program:

	const glResources = initGlResources(gl, textureImages);

	// Instantiate and run the WebAssembly module.

	runWasm(gl, glResources, wasm);
}

// Convert Javascript's key names into numeric codes for use in the Rust program

const keymap = {
	Escape: 27,
	Space: 32,
	ArrowLeft: 37,
	ArrowUp: 38,
	ArrowRight: 39,
	ArrowDown: 40,
	Digit0: 48,
	Digit1: 49,
	Digit2: 50,
	Digit3: 51,
	Digit4: 52,
	Digit5: 53,
	Digit6: 54,
	Digit7: 55,
	Digit8: 56,
	Digit9: 57,
	KeyA: 65,
	KeyB: 66,
	KeyC: 67,
	KeyD: 68,
	KeyE: 69,
	KeyF: 70,
	KeyG: 71,
	KeyH: 72,
	KeyI: 73,
	KeyJ: 74,
	KeyK: 75,
	KeyL: 76,
	KeyM: 77,
	KeyN: 78,
	KeyO: 79,
	KeyP: 80,
	KeyQ: 81,
	KeyR: 82,
	KeyS: 83,
	KeyT: 84,
	KeyU: 85,
	KeyV: 86,
	KeyW: 87,
	KeyX: 88,
	KeyY: 89,
	KeyZ: 90,
	Numpad1: 97,
	Numpad2: 98,
	Numpad3: 99,
	Numpad4: 100,
	Numpad5: 101,
	Numpad6: 102,
	Numpad7: 103,
	Numpad8: 104,
	Numpad9: 105,
	Period: 190,
	Slash: 191,
};

function runWasm(gl, glResources, wasm) {

	let screenValid = false;

	let importObject = {
		env: {
			js_draw_rect: function(dst_x, dst_y, size_x, size_y, color) {
				drawTile(gl, glResources, dst_x, dst_y, size_x, size_y, color, glResources.textures.length - 1, 0, 0);
			},
			js_draw_tile: function(dst_x, dst_y, size_x, size_y, color, textureIndex, src_x, src_y) {
				drawTile(gl, glResources, dst_x, dst_y, size_x, size_y, color, textureIndex, src_x, src_y);
			},
			js_invalidate_screen: function() {
				screenValid = false;
			}
		},
	};

	wasm.arrayBuffer().then(bytes =>
		WebAssembly.instantiate(bytes, importObject)
	).then(result => {
		const wasmExports = result.instance.exports;

		function ensureScreenValid() {
			if (!screenValid) {
				drawScreen(gl, glResources, wasmExports.rs_on_draw);
				screenValid = true;
			}
		}

		const seed0 = Math.random() * 2**32;
		const seed1 = Math.random() * 2**32;

		wasmExports.rs_start(seed0, seed1);
		ensureScreenValid();

		document.body.addEventListener('keydown', e => {
			const key = keymap[e.code] || null;
			// console.log("Key Pressed:" + e.key + " (" + e.code + ") -> " + key);
			if (key != null) {
				e.preventDefault();
				wasmExports.rs_on_key_down(key, e.ctrlKey, e.shiftKey);
				ensureScreenValid();
			}
		});
	});
}

function initGlResources(gl, textureImages) {
	const vsSource = `
		attribute vec4 aVertexPosition;
		attribute vec4 aVertexColor;
		
		uniform mat4 uProjectionMatrix;

		varying lowp vec4 vColor;
		varying highp vec2 vTextureCoord;

		void main() {
			gl_Position = uProjectionMatrix * vec4(aVertexPosition.xy, 0, 1);
			vColor = aVertexColor;
			vTextureCoord = aVertexPosition.zw;
		}
	`;

	const fsSource = `
		varying lowp vec4 vColor;
		varying highp vec2 vTextureCoord;

		uniform sampler2D uSampler;

		void main() {
			gl_FragColor = vColor * texture2D(uSampler, vTextureCoord);
		}
	`;

	const program = initShaderProgram(gl, vsSource, fsSource);

	const buffers = initBuffers(gl);

	const textures = textureImages.map(image => createTextureFromImage(gl, image));
	textures.push(createTextureFromColor(gl, 255, 255, 255, 255));

	const glResources = {
		program: program,
		attribLocations: {
			vertexPosition: gl.getAttribLocation(program, 'aVertexPosition'),
			vertexColor: gl.getAttribLocation(program, 'aVertexColor'),
		},
		uniformLocations: {
			projectionMatrix: gl.getUniformLocation(program, 'uProjectionMatrix'),
			uSampler: gl.getUniformLocation(program, 'uSampler'),
		},
		buffers: buffers,
		textures: textures,
		currentTextureIndex: 0,
	};

	gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
	gl.enable(gl.BLEND);
	gl.clearColor(0, 0, 0, 1.0);

	gl.bindBuffer(gl.ARRAY_BUFFER, glResources.buffers.position);
	gl.vertexAttribPointer(glResources.attribLocations.vertexPosition, 4, gl.FLOAT, false, 0, 0);
	gl.enableVertexAttribArray(glResources.attribLocations.vertexPosition);

	gl.bindBuffer(gl.ARRAY_BUFFER, glResources.buffers.color);
	gl.vertexAttribPointer(glResources.attribLocations.vertexColor, 4, gl.UNSIGNED_BYTE, true, 0, 0);
	gl.enableVertexAttribArray(glResources.attribLocations.vertexColor);

	gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, glResources.buffers.indices);

	gl.useProgram(glResources.program);

	gl.activeTexture(gl.TEXTURE0);
	gl.bindTexture(gl.TEXTURE_2D, glResources.textures[glResources.currentTextureIndex]);
	gl.uniform1i(glResources.uniformLocations.uSampler, 0);

	return glResources;
}

function initBuffers(gl) {
	return {
		position: gl.createBuffer(),
		color: gl.createBuffer(),
		indices: createElementBuffer(gl),
	};
}

function createElementBuffer(gl) {

	const indices = new Uint16Array(maxQuads * 6);

	for (let i = 0; i < maxQuads; ++i) {
		let j = 6*i;
		let k = 4*i;
		indices[j+0] = k+0;
		indices[j+1] = k+1;
		indices[j+2] = k+2;
		indices[j+3] = k+2;
		indices[j+4] = k+1;
		indices[j+5] = k+3;
	}

	const indexBuffer = gl.createBuffer();
	gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
	gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);
	
	return indexBuffer;
}

function drawScreen(gl, glResources, drawFunc) {
	const screenX = gl.canvas.clientWidth;
	const screenY = gl.canvas.clientHeight;

	projectionMatrix[0] = 2 / screenX;
	projectionMatrix[5] = 2 / screenY;

	gl.clear(gl.COLOR_BUFFER_BIT);

	gl.uniformMatrix4fv(glResources.uniformLocations.projectionMatrix, false, projectionMatrix);
	
	drawFunc(screenX, screenY);

	renderQuads(gl, glResources);
}

function initShaderProgram(gl, vsSource, fsSource) {
	const vertexShader = loadShader(gl, gl.VERTEX_SHADER, vsSource);
	const fragmentShader = loadShader(gl, gl.FRAGMENT_SHADER, fsSource);

	const program = gl.createProgram();
	gl.attachShader(program, vertexShader);
	gl.attachShader(program, fragmentShader);
	gl.linkProgram(program);

	if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
		alert('Unable to initialize the shader program: ' + gl.getProgramInfoLog(program));
		return null;
	}

	return program;
}

function loadShader(gl, type, source) {
	const shader = gl.createShader(type);

	gl.shaderSource(shader, source);
	gl.compileShader(shader);

	if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
		alert('An error occurred compiling the shaders: ' + gl.getShaderInfoLog(shader));
		gl.deleteShader(shader);
		return null;
	}

	return shader;
}

function createTextureFromColor(gl, r, g, b, a) {
	const texture = gl.createTexture();
	gl.bindTexture(gl.TEXTURE_2D, texture);

	const level = 0;
	const internalFormat = gl.RGBA;
	const width = 1;
	const height = 1;
	const border = 0;
	const srcFormat = gl.RGBA;
	const srcType = gl.UNSIGNED_BYTE;
	const pixel = new Uint8Array([r, g, b, a]);
	gl.texImage2D(gl.TEXTURE_2D, level, internalFormat,
				  width, height, border, srcFormat, srcType,
				  pixel);
	return texture;
}

function createTextureFromImage(gl, image) {
	const texture = gl.createTexture();
	gl.bindTexture(gl.TEXTURE_2D, texture);

	const level = 0;
	const internalFormat = gl.RGBA;
	const srcFormat = gl.RGBA;
	const srcType = gl.UNSIGNED_BYTE;
//	gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
	gl.texImage2D(gl.TEXTURE_2D, level, internalFormat, srcFormat, srcType, image);
	gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
	gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
	gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
	gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

	return texture;
}

function renderQuads(gl, glResources) {
	if (numQuads > 0) {
		gl.bindBuffer(gl.ARRAY_BUFFER, glResources.buffers.position);
		gl.bufferData(gl.ARRAY_BUFFER, vertexPositions, gl.DYNAMIC_DRAW);

		gl.bindBuffer(gl.ARRAY_BUFFER, glResources.buffers.color);
		gl.bufferData(gl.ARRAY_BUFFER, vertexColors, gl.DYNAMIC_DRAW);

		gl.drawElements(gl.TRIANGLES, 6 * numQuads, gl.UNSIGNED_SHORT, 0);
	}
	numQuads = 0;
}

function drawTile(gl, glResources, destX, destY, sizeX, sizeY, color, textureIndex, srcX, srcY) {
	const x0 = destX;
	const y0 = destY;
	const x1 = destX + sizeX;
	const y1 = destY + sizeY;
	const texSizeX = 256; // hard-coded; should change
	const texSizeY = 256; // hard-coded; should change
	// Note: Flipping vertically here by swapping the expected values for t0 and t1
	const s0 = srcX / texSizeX;
	const t0 = (srcY + sizeY) / texSizeY;
	const s1 = (srcX + sizeX) / texSizeX;
	const t1 = srcY / texSizeY;
	addQuad(gl, glResources, x0, y0, x1, y1, s0, t0, s1, t1, color, textureIndex);
}

function addQuad(gl, glResources, x0, y0, x1, y1, s0, t0, s1, t1, color, textureIndex) {
	textureIndex = Math.max(0, Math.min(glResources.textures.length - 1, textureIndex));
	if (textureIndex != glResources.currentTextureIndex) {
		// Texture is different; render and change textures
		renderQuads(gl, glResources);
		glResources.currentTextureIndex = textureIndex;
		gl.bindTexture(gl.TEXTURE_2D, glResources.textures[glResources.currentTextureIndex]);
	} else if (numQuads >= maxQuads) {
		// Buffer is full; render
		renderQuads(gl, glResources);
	}

	// Append four vertices to the position/texcoord and color arrays

	const i = numQuads * 16;

	vertexPositions[i+0] = x0;
	vertexPositions[i+1] = y0;
	vertexPositions[i+2] = s0;
	vertexPositions[i+3] = t0;

	vertexPositions[i+4] = x1;
	vertexPositions[i+5] = y0;
	vertexPositions[i+6] = s1;
	vertexPositions[i+7] = t0;

	vertexPositions[i+8] = x0;
	vertexPositions[i+9] = y1;
	vertexPositions[i+10] = s0;
	vertexPositions[i+11] = t1;

	vertexPositions[i+12] = x1;
	vertexPositions[i+13] = y1;
	vertexPositions[i+14] = s1;
	vertexPositions[i+15] = t1;

	for (let j = numQuads * 4; j < (numQuads + 1) * 4; ++j) {
		vertexColors[j] = color;
	}

	++numQuads;
}

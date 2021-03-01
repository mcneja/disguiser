var width = 80;
var height = 60;
var squareSize = 12;
var c = document.querySelector("#canvas");
c.width = width*squareSize;
c.height = height*squareSize;

var ctx = c.getContext('2d');
ctx.textAlign = 'center';
ctx.font = '12px arial';

// Convert Javascript's key names into numeric codes for use in the Rust program
const keymap = {
	ArrowLeft: 37,
	Numpad4: 37,
	ArrowUp: 38,
	Numpad8: 38,
	ArrowRight: 39,
	Numpad6: 39,
	ArrowDown: 40,
	Numpad2: 40,
};

fetch('roguelike.wasm')
.then(response => response.arrayBuffer())
.then(bytes => WebAssembly.instantiate(bytes, {
  env: {
   put_character: function(x,y,char,color) {
     ctx.fillStyle = '#' + color.toString(16).padStart(6, '0');
     ctx.clearRect(x*squareSize, y*squareSize, squareSize,squareSize);
     ctx.fillText(String.fromCharCode(char), x*squareSize+squareSize/2, y*squareSize+squareSize/2);
   }
 }
}))
.then(results => {
    results.instance.exports.start(width,height);
    document.body.addEventListener("keydown",function(e){
	  let key = keymap[e.code] || null
      console.log("Key Pressed:"+e.key+" ("+e.code+") -> "+key);
	  if (key != null) {
        results.instance.exports.key_down(key);
      }
    })
});

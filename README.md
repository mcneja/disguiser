# What is this?

A Seven-day Roguelike by James McNeill. Written for the 2021 7DRL challenge.

Explore mansions, steal all their loot, and get out without being caught by the guards.
Don disguises to avoid detection.

## How to build

    cargo build --release --target wasm32-unknown-unknown
    
Copy the resulting Webassembly file into the web-serving directory and change its name:

    From: target/wasm32-unknown-unknown/release/thiefrl2_wasm.wasm
    To: web/game.wasm

Run a web server from the web directory; for instance if Python is installed you can use

    python -m http.server --directory web

I think Cargo has a web server like this as well but Python is the one I was familiar with.

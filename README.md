# What is this?

A port of ThiefRL2, a 2016 7DRL (Seven Day Roguelike) challenge entry by James McNeill.

The original was written in C++ using OpenGL and Win32. This version is written in Javascript and Rust.

Explore mansions, steal all their loot, and get out without being caught by the guards.

## How to build

    cargo build --release --target wasm32-unknown-unknown
    
Copy the resulting Webassembly file into the web-serving directory and change its name:

    From: target/wasm32-unknown-unknown/release/thiefrl2_wasm.wasm
    To: web/game.wasm

Run a web server from the web directory; for instance if Python is installed you can use

    python -m http.server --directory web

I think Cargo has a web server like this as well but Python is the one I was familiar with.

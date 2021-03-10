@echo off
cargo build --release --target wasm32-unknown-unknown
copy target\wasm32-unknown-unknown\release\disguiser.wasm web\game.wasm

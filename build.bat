@echo off
cargo build --release --target wasm32-unknown-unknown
copy target\wasm32-unknown-unknown\release\thiefrl2_wasm.wasm web\game.wasm

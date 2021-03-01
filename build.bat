@echo off
pushd roguelike
cargo build --release --target wasm32-unknown-unknown
copy target\wasm32-unknown-unknown\release\roguelike.wasm ..
popd

@echo off
pushd roguelike
cargo build --release --target wasm32-unknown-unknown
popd
copy roguelike\target\wasm32-unknown-unknown\release\roguelike.wasm web\

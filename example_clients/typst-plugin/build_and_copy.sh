# make crashes apparent
set -o errexit

cargo build --release --target wasm32-unknown-unknown
cp ../../target/wasm32-unknown-unknown/release/typst_plugin.wasm .
wasm-opt typst_plugin.wasm -o typst_plugin_opt.wasm -O
mv typst_plugin_opt.wasm typst_plugin.wasm

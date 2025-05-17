# build #[wasm_bindgen] JS imports as basic JavaScript module into `simple-semantics-js/wbg`
wasm-pack build -t web -d "../simple-semantics-js/wbg" simple_semantics_ffi/

# run diplomat-tool for #[diplomat::bridge] modules
diplomat-tool -e simple_semantics_ffi/src/lib.rs js "simple-semantics-js/api"

# fix diplomat generated code
cp simple-semantics-js/diplomat-wasm.mjs.template simple-semantics-js/api/diplomat-wasm.mjs

# fix wasm-bindgen generated code
# remove 
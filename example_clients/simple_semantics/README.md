# Simple semantics client example

This client is structured into three parts:
- `simple_semantics`: A pure Rust library defining the semantics of the language
- `simple_semantics_ffi`: A bridge Rust library that exposes `simple_semantics` to FFI using `wasm-bindgen` and `diplomat`
- `www`: A node.js application that uses the generated WASM binary and JavaScript modules from `simple_semantics_ffi` to provide a web interface

Any changes should mostly happen in the Rust files and in the node.js application.

For changes to Rust files, run `bash build.sh` to build the WASM binary and JavaScript modules.

Changes to the node.js application don't require any further build steps apart from running using `npm run start`.

# TODO: figure out if combining wasm-bindgen and diplomat like this is fine.
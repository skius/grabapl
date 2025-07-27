# Template for writing a grabapl semantics and client

This directory contains a documented example of how to write a grabapl semantics and optionally use it over FFI.

## Components

### Rust

The `semantics` directory defines the pure Rust semantics of your specific client.
For more details, see its rendered documentation: https://docs.rs/grabapl_template_semantics/latest/grabapl_template_semantics/.
If you don't want to use the library over FFI, this is all you need.

### FFI

The `ffi` directory contains example code that uses [Diplomat](https://github.com/rust-diplomat/diplomat/) to automatically
generate bindings to the library for other languages.
For more details, see its rendered documentation: https://docs.rs/grabapl_template_ffi/latest/grabapl_template_ffi/.

The `js` directory contains the generated JavaScript bindings (via WASM) for the FFI library.

The `www` directory contains a simple NPM project that uses the generated JavaScript bindings to run the library in a browser via WebAssembly.

#### Building the FFI library

You are free to use whichever FFI mechanism you prefer, but for this example, you need the following CLI tools installed:
- [`diplomat-tool`](https://github.com/rust-diplomat/diplomat?tab=readme-ov-file#installation)
- [`wasm-bindgen`](https://github.com/wasm-bindgen/wasm-bindgen?tab=readme-ov-file#install-wasm-bindgen-cli)
- [`wasm-opt`](https://github.com/WebAssembly/binaryen#tools)

Once those tools are installed, run `build.sh` in the root of this directory to build the example `.wasm` module and to generate the JavaScript bindings for it.

#### Using the FFI library

See [`www/README.md`](www/README.md) for how to run the example JavaScript project.
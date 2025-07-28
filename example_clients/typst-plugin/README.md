# Grabapl Typst Plugin
Exposes a function (via WASM) that parses grabapl source with a show_state instruction
and returns the abstract graph shown as DOT.

## Notes
https://rustwasm.github.io/docs/book/game-of-life/code-size.html

not using debug information reduces size from ~35MB to ~1.4MB

wasm-opt -Oz further reduces that to ~950KB


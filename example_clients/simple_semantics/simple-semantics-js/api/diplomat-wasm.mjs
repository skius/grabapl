import cfg from '../diplomat.config.mjs';
import {readString8} from './diplomat-runtime.mjs'
import * as wbg from '../wbg/simple_semantics_ffi.js'

const imports = {
    env: {
        diplomat_console_debug_js(ptr, len) {
            console.debug(readString8(wasm, ptr, len));
        },
        diplomat_console_error_js(ptr, len) {
            console.error(readString8(wasm, ptr, len));
        },
        diplomat_console_info_js(ptr, len) {
            console.info(readString8(wasm, ptr, len));
        },
        diplomat_console_log_js(ptr, len) {
            console.log(readString8(wasm, ptr, len));
        },
        diplomat_console_warn_js(ptr, len) {
            console.warn(readString8(wasm, ptr, len));
        },
        diplomat_throw_error_js(ptr, len) {
            throw new Error(readString8(wasm, ptr, len));
        }
    },
    wbg: wbg.__wbg_get_imports().wbg
}

let wasm = await wbg.default(imports)

wasm.diplomat_init();
if (cfg['init'] !== undefined) {
    cfg['init'](wasm);
}

export default wasm;

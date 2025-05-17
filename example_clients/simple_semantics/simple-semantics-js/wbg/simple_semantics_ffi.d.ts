/* tslint:disable */
/* eslint-disable */

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly AbstractGraph_destroy: (a: number) => void;
  readonly ConcreteGraph_create: () => number;
  readonly ConcreteGraph_add_node: (a: number, b: number) => number;
  readonly ConcreteGraph_add_edge: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly ConcreteGraph_say_hi: (a: number) => void;
  readonly ConcreteGraph_destroy: (a: number) => void;
  readonly DotCollector_create: () => number;
  readonly DotCollector_collect: (a: number, b: number) => void;
  readonly DotCollector_get_dot: (a: number, b: number) => void;
  readonly DotCollector_destroy: (a: number) => void;
  readonly diplomat_init: () => void;
  readonly diplomat_simple_write: (a: number, b: number, c: number) => void;
  readonly diplomat_buffer_write_create: (a: number) => number;
  readonly diplomat_buffer_write_get_bytes: (a: number) => number;
  readonly diplomat_buffer_write_len: (a: number) => number;
  readonly diplomat_buffer_write_destroy: (a: number) => void;
  readonly diplomat_alloc: (a: number, b: number) => number;
  readonly diplomat_free: (a: number, b: number, c: number) => void;
  readonly diplomat_is_str: (a: number, b: number) => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

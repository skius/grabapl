/* tslint:disable */
/* eslint-disable */

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly CompileResult_dot_of_state: (a: number, b: number, c: number, d: number) => void;
  readonly CompileResult_get_program: (a: number, b: number) => void;
  readonly CompileResult_destroy: (a: number) => void;
  readonly ConcreteGraph_create: () => number;
  readonly ConcreteGraph_dot: (a: number, b: number) => void;
  readonly ConcreteGraph_add_node: (a: number, b: number, c: number, d: number) => void;
  readonly ConcreteGraph_add_edge: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly ConcreteGraph_destroy: (a: number) => void;
  readonly Grabapl_init: () => void;
  readonly Grabapl_parse: (a: number, b: number) => number;
  readonly Grabapl_destroy: (a: number) => void;
  readonly OperationBuilder_create: (a: number, b: number) => number;
  readonly OperationBuilder_expect_parameter_node: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly OperationBuilder_destroy: (a: number) => void;
  readonly OperationContext_create: () => number;
  readonly OperationContext_destroy: (a: number) => void;
  readonly Program_op_ctx: (a: number) => number;
  readonly Program_run_operation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly Program_destroy: (a: number) => void;
  readonly StringError_to_string: (a: number, b: number) => void;
  readonly StringError_destroy: (a: number) => void;
  readonly diplomat_init: () => void;
  readonly diplomat_simple_write: (a: number, b: number, c: number) => void;
  readonly diplomat_buffer_write_create: (a: number) => number;
  readonly diplomat_buffer_write_get_bytes: (a: number) => number;
  readonly diplomat_buffer_write_len: (a: number) => number;
  readonly diplomat_buffer_write_destroy: (a: number) => void;
  readonly diplomat_alloc: (a: number, b: number) => number;
  readonly diplomat_free: (a: number, b: number, c: number) => void;
  readonly diplomat_is_str: (a: number, b: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_3: WebAssembly.Table;
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

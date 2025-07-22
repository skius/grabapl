/* tslint:disable */
/* eslint-disable */

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly ConcreteGraph_new: () => number;
  readonly ConcreteGraph_add_node: (a: number, b: number, c: number) => number;
  readonly ConcreteGraph_add_edge: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly ConcreteGraph_get_nodes: (a: number) => number;
  readonly ConcreteGraph_get_edges: (a: number) => number;
  readonly ConcreteGraph_destroy: (a: number) => void;
  readonly Context_init: () => void;
  readonly Context_parse: (a: number, b: number) => number;
  readonly EdgeWrapper_src: (a: number) => number;
  readonly EdgeWrapper_dst: (a: number) => number;
  readonly EdgeWrapper_weight: (a: number, b: number) => void;
  readonly EdgeWrapper_destroy: (a: number) => void;
  readonly EdgesIter_next: (a: number) => number;
  readonly EdgesIter_to_iterable: (a: number) => number;
  readonly EdgesIter_destroy: (a: number) => void;
  readonly LineColSpansIter_next: (a: number, b: number) => void;
  readonly LineColSpansIter_to_iterable: (a: number) => number;
  readonly LineColSpansIter_destroy: (a: number) => void;
  readonly NewNode_key: (a: number) => number;
  readonly NewNode_name: (a: number, b: number) => void;
  readonly NewNode_value: (a: number, b: number) => void;
  readonly NewNode_destroy: (a: number) => void;
  readonly NewNodesIter_next: (a: number) => number;
  readonly NewNodesIter_to_iterable: (a: number) => number;
  readonly NewNodesIter_destroy: (a: number) => void;
  readonly NodeWrapper_key: (a: number) => number;
  readonly NodeWrapper_value: (a: number, b: number) => void;
  readonly NodeWrapper_destroy: (a: number) => void;
  readonly NodesIter_next: (a: number) => number;
  readonly NodesIter_to_iterable: (a: number) => number;
  readonly NodesIter_destroy: (a: number) => void;
  readonly OpCtxAndFnNames_destroy: (a: number) => void;
  readonly ParseResult_error_message: (a: number, b: number) => void;
  readonly ParseResult_error_spans: (a: number) => number;
  readonly ParseResult_dot_of_state: (a: number, b: number, c: number, d: number) => void;
  readonly ParseResult_list_states: (a: number) => number;
  readonly ParseResult_list_operations: (a: number) => number;
  readonly ParseResult_run_operation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly ParseResult_destroy: (a: number) => void;
  readonly StringError_to_string: (a: number, b: number) => void;
  readonly StringError_destroy: (a: number) => void;
  readonly StringIter_next: (a: number) => number;
  readonly StringIter_to_iterable: (a: number) => number;
  readonly StringIter_destroy: (a: number) => void;
  readonly StringWrapper_new: (a: number, b: number) => number;
  readonly StringWrapper_to_string: (a: number, b: number) => void;
  readonly StringWrapper_destroy: (a: number) => void;
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

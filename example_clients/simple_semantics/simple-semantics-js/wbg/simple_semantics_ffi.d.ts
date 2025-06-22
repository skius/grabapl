/* tslint:disable */
/* eslint-disable */

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly AbstractArgList_create: () => number;
  readonly AbstractArgList_push: (a: number, b: number) => void;
  readonly AbstractArgList_destroy: (a: number) => void;
  readonly AbstractGraph_destroy: (a: number) => void;
  readonly AbstractNodeId_new_parameter: (a: number) => number;
  readonly AbstractNodeId_new_from_output: (a: number, b: number, c: number, d: number) => number;
  readonly AbstractNodeId_destroy: (a: number) => void;
  readonly BuilderOpLike_new_from_id: (a: number) => number;
  readonly BuilderOpLike_destroy: (a: number) => void;
  readonly BuiltinQuery_new_is_value_gt: (a: number) => number;
  readonly BuiltinQuery_new_is_value_eq: (a: number) => number;
  readonly BuiltinQuery_new_values_equal: () => number;
  readonly BuiltinQuery_new_first_gt_snd: () => number;
  readonly BuiltinQuery_destroy: (a: number) => void;
  readonly ConcreteGraph_create: () => number;
  readonly ConcreteGraph_add_node: (a: number, b: number) => number;
  readonly ConcreteGraph_add_edge: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly ConcreteGraph_say_hi: (a: number) => void;
  readonly ConcreteGraph_destroy: (a: number) => void;
  readonly DotCollector_create: () => number;
  readonly DotCollector_collect: (a: number, b: number) => void;
  readonly DotCollector_get_dot: (a: number, b: number) => void;
  readonly DotCollector_destroy: (a: number) => void;
  readonly EdgeAbstract_new_wildcard: () => number;
  readonly EdgeAbstract_new_exact: (a: number, b: number) => number;
  readonly EdgeAbstract_destroy: (a: number) => void;
  readonly IntermediateState_get_dot: (a: number, b: number) => void;
  readonly IntermediateState_available_aids: (a: number, b: number) => void;
  readonly IntermediateState_destroy: (a: number) => void;
  readonly OpCtx_create: () => number;
  readonly OpCtx_destroy: (a: number) => void;
  readonly OperationBuilder_create: (a: number) => number;
  readonly OperationBuilder_expect_parameter_node: (a: number, b: number, c: number) => void;
  readonly OperationBuilder_expect_context_node: (a: number, b: number, c: number) => void;
  readonly OperationBuilder_expect_parameter_edge: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly OperationBuilder_start_query: (a: number, b: number, c: number, d: number) => void;
  readonly OperationBuilder_enter_true_branch: (a: number, b: number) => void;
  readonly OperationBuilder_enter_false_branch: (a: number, b: number) => void;
  readonly OperationBuilder_start_shape_query: (a: number, b: number, c: number, d: number) => void;
  readonly OperationBuilder_end_query: (a: number, b: number) => void;
  readonly OperationBuilder_expect_shape_node: (a: number, b: number, c: number, d: number) => void;
  readonly OperationBuilder_expect_shape_edge: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly OperationBuilder_add_instruction: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => void;
  readonly OperationBuilder_show: (a: number, b: number) => void;
  readonly OperationBuilder_destroy: (a: number) => void;
  readonly OperationBuilderError_message: (a: number, b: number) => void;
  readonly OperationBuilderError_destroy: (a: number) => void;
  readonly Runner_create: () => number;
  readonly Runner_run: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly Runner_destroy: (a: number) => void;
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

//! This FFI crate exposes any functionality of [`grabapl`] and the custom semantics to other languages.
//!
//! This example uses the [Diplomat] tool to automatically generate idiomatic FFI bindings to
//! multiple target languages.
//!
//! See the main `README.md` for information on how to build this crate and integrate it
//! into a different language project.
//!
//! For more inspiration on FFI crates, the other clients in the `example_clients` directory.
//!
//! [Diplomat]: https://github.com/rust-diplomat/diplomat/

use grabapl::operation::builder::IntermediateState;
use grabapl::prelude::*;
use grabapl_syntax::custom_syntax::CustomSyntax;
use semantics::*;
use grabapl_syntax::interpreter::lex_then_parse;

type RustOperationContext = OperationContext<TheSemantics>;
type RustIntermediateState = IntermediateState<TheSemantics>;
type RustConcreteGraph = ConcreteGraph<TheSemantics>;
type RustOperationBuilder<'a> = OperationBuilder<'a, TheSemantics>;

fn parse_node_value(s: &str) -> Option<NodeValue> {
    let parser = syntax::node_value_parser();
    lex_then_parse(s, parser).ok()
}

fn parse_edge_value(s: &str) -> Option<EdgeValue> {
    let parser = syntax::edge_value_parser();
    lex_then_parse(s, parser).ok()
}

fn parse_node_type(s: &str) -> Result<NodeType, String> {
    let parser = syntax::TheCustomSyntax::get_node_type_parser();
    lex_then_parse(s, parser).map_err(|e| e.to_string())
}

fn parse_edge_type(s: &str) -> Result<EdgeType, String> {
    let parser = syntax::TheCustomSyntax::get_edge_type_parser();
    lex_then_parse(s, parser).map_err(|e| e.to_string())
}

/// This module is sent to Diplomat to automatically generate FFI bindings from functions and types
/// on both the Rust and the target language side.
///
/// See [The Diplomat Book] for detailed information on how and which types and functions can be
/// exposed with Diplomat.
///
/// In general, we will create a `diplomat::opaque` wrapper type for every type we want to expose,
/// which must be created with a `Box<Self>` return type.
///
/// [The Diplomat Book]: https://rust-diplomat.github.io/diplomat/
#[diplomat::bridge]
mod ffi {
    use std::collections::HashMap;
    // we need to import this to use the write! macro
    use std::fmt::Write as _;
    use error_stack::fmt::ColorMode;
    use grabapl::NodeKey;
    use super::{OperationId, RustConcreteGraph};
    use super::RustOperationContext;
    use super::TheSemantics;
    use super::RustIntermediateState;
    use super::RustOperationBuilder;

    /// Holds a bunch of top-level functions.
    #[diplomat::opaque]
    pub struct Grabapl;

    impl Grabapl {
        /// Call this function at the beginning of your program to initialize useful
        /// Rust panic error messages.
        pub fn init() {
            // NOTE: without this call, the "error: failed to find intrinsics to enable `clone_ref` function" error
            //  may be issued by `wasm-bindgen`.
            // Most likely a bug in `wasm-bindgen`.
            console_error_panic_hook::set_once();
            // change error-stack's color mode
            error_stack::Report::set_color_mode(ColorMode::None);
            // print something to console to indicate that the library has been initialized
            log::info!("Grabapl FFI initialized");
        }

        /// Parses a source file.
        pub fn parse(src: &str) -> Box<CompileResult> {
            let raw_res = grabapl_syntax::try_parse_to_op_ctx_and_map(src, false /* disable colored error messages - see the online_syntax demo for how to handle them */);
            let op_ctx_and_map_res = raw_res.op_ctx_and_map
                // turn function names into owned strings
                .map(|(op_ctx, map)| {
                    let state_map = map.into_iter()
                        .map(|(k, v)| (k.into(), v))
                        .collect::<HashMap<String, _>>();
                    (op_ctx, state_map)
                })
                // project away line/col spans for brevity
                .map_err(|e| e.value);

            Box::new(CompileResult {
                op_ctx_and_map_res,
                state_map: raw_res.state_map,
            })
        }
    }

    /// Represents a concrete graph, i.e., the runtime state of a program.
    #[diplomat::opaque]
    pub struct ConcreteGraph(RustConcreteGraph);

    impl ConcreteGraph {
        /// Creates a new empty concrete graph.
        pub fn create() -> Box<ConcreteGraph> {
            Box::new(ConcreteGraph(RustConcreteGraph::new()))
        }

        /// Returns the DOT representation of the concrete graph.
        pub fn dot(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.0.dot()).unwrap();
        }

        /// Adds a new node to the concrete graph with the given value and returns its key.
        pub fn add_node(&mut self, value: &str) -> Result<u32, Box<StringError>> {
            let node_value = super::parse_node_value(value)
                .ok_or_else(|| StringError::from_boxed(format!("Invalid node value: {}", value)))?;
            let node_key = self.0.add_node(node_value);
            Ok(node_key.0)
        }

        /// Adds an edge from the node with key `from` to the node with key `to` with the given value.
        pub fn add_edge(
            &mut self,
            from: u32,
            to: u32,
            value: &str,
        ) -> Result<(), Box<StringError>> {
            let edge_value = super::parse_edge_value(value)
                .ok_or_else(|| StringError::from_boxed(format!("Invalid edge value: {}", value)))?;
            let from_key = NodeKey(from);
            let to_key = NodeKey(to);
            self.0.add_edge(from_key, to_key, edge_value);
            Ok(())
        }
    }

    /// The operation context maps operation IDs (integers) to operations.
    #[diplomat::opaque]
    pub struct OperationContext(RustOperationContext);

    impl OperationContext {
        /// Creates a new operation context.
        pub fn create() -> Box<OperationContext> {
            let op_ctx = RustOperationContext::new();
            // here you could populate op_ctx with a bunch of default builtin operations.
            // in general, a user will probably want to specify the "const-generic" arguments
            // of a builtin operation, like the constant to add in `TheOperation::AddConstant`,
            // but they cannot do that here, since we have to pick a specific constant in order
            // to add the operation. Hence calling builtin operations will typically involve
            // explicit construction of the builtin operation to call and bypassing operation IDs.
            // User defined operation will always be called via operation IDs and this context, however.
            Box::new(OperationContext(op_ctx))
        }
    }

    /// Represents the result of compiling a source file.
    ///
    /// The compilation may have failed, but there may still be valid intermediate states to print.
    /// To check for errors and access the programs, call getProgram().
    #[diplomat::opaque]
    pub struct CompileResult {
        op_ctx_and_map_res: Result<(RustOperationContext, HashMap<String, OperationId>), String>,
        state_map: HashMap<String, RustIntermediateState>,
    }

    impl CompileResult {
        /// Returns the DOT representation of the intermediate state named `state`.
        pub fn dot_of_state(&self, state: &str, dot_out: &mut DiplomatWrite) {
            let Some(state) = self.state_map.get(state) else {
                log::error!("state does not exist in state map");
                return;
            };
            write!(dot_out, "{}", state.dot_with_aid()).unwrap();
        }

        /// If the compilation was successful, this returns a `Program` that can be used to run operations.
        ///
        /// Otherwise, this throws an error.
        pub fn get_program(&self) -> Result<Box<Program>, Box<StringError>> {
            match &self.op_ctx_and_map_res {
                Ok((op_ctx, fn_map)) => {
                    let program = Program {
                        op_ctx: op_ctx.clone(),
                        fn_map: fn_map.clone(),
                    };
                    Ok(Box::new(program))
                }
                Err(err) => Err(Box::new(StringError(err.to_string()))),
            }
        }
    }

    /// Represents a program that can be executed.
    #[diplomat::opaque]
    pub struct Program {
        op_ctx: RustOperationContext,
        fn_map: HashMap<String, OperationId>,
    }

    impl Program {
        /// Returns a copy of the operation context.
        pub fn op_ctx(&self) -> Box<OperationContext> {
            Box::new(OperationContext(self.op_ctx.clone()))
        }

        /// Runs the operation with the given name on the provided concrete graph with the given arguments.
        pub fn run_operation(
            &self,
            g: &mut ConcreteGraph,
            op_name: &str,
            args: &[u32],
        ) -> Result<(), Box<StringError>> {
            let op_id = self.fn_map.get(op_name)
                .ok_or_else(|| StringError(format!("Operation '{}' not found", op_name)))?;
            let args: Vec<_> = args.iter().copied().map(NodeKey).collect();
            let res = super::run_from_concrete(&mut g.0, &self.op_ctx, *op_id, &args);
            res.map_err(|e| Box::new(StringError(e.to_string()))).map(|_| ())
        }
    }

    /// A user defined operation that is currently being built using low-level instructions instead of
    /// parsing via the syntax parser.
    ///
    /// This builder should probably be used to create an interactive interface for building user defined operations.
    #[diplomat::opaque]
    pub struct OperationBuilder<'a>(RustOperationBuilder<'a>);

    impl<'a> OperationBuilder<'a> {
        /// Creates a new operation builder for the given operation context and with the given self operation ID.
        ///
        /// The passed operation context holds the other user defined operations that can be used in the builder.
        pub fn create(op_ctx: &'a OperationContext, self_op_id: u32) -> Box<OperationBuilder<'a>> {
            let op_builder = RustOperationBuilder::new(&op_ctx.0, self_op_id);
            Box::new(OperationBuilder(op_builder))
        }

        /// Adds an expected parameter node with the given name and type to the operation.
        pub fn expect_parameter_node(&mut self, name: &str, node_type: &str) -> Result<(), Box<StringError>> {
            let node_type = super::parse_node_type(node_type).map_err(|e| StringError::from_boxed(format!("Invalid node type: {}", e)))?;
            self.0.expect_parameter_node(name, node_type).map_err(|e| StringError::from_boxed(e.to_string()))
        }

        // TODO: add more of the desired builder operations here. See the main `OperationBuilder` documentation.
    }


    /// Catch this in a try-catch and print it with toString().
    #[diplomat::opaque]
    pub struct StringError(String);

    impl StringError {
        fn from_boxed(s: String) -> Box<StringError> {
            Box::new(StringError(s))
        }

        #[diplomat::attr(auto, stringifier)]
        pub fn to_string(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.0).unwrap();
        }
    }

}
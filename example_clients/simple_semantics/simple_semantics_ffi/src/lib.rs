use wasm_bindgen::prelude::wasm_bindgen;
use grabapl::Semantics;
use simple_semantics::SimpleSemantics;

#[wasm_bindgen]
extern "C" {
    fn prompt(s: &str) -> String;
}

type RustEdgeAbstract = <SimpleSemantics as Semantics>::EdgeAbstract;
type RustNodeAbstract = <SimpleSemantics as Semantics>::NodeAbstract;
type RustNodeConcrete = <SimpleSemantics as Semantics>::NodeConcrete;
type RustEdgeConcrete = <SimpleSemantics as Semantics>::EdgeConcrete;

use grabapl::SubstMarker;

#[diplomat::bridge]
mod ffi {
    use super::prompt;
    use ::grabapl::{Semantics};
    use ::grabapl::graph::semantics::AbstractGraph as RustAbstractGraph;
    use ::grabapl::graph::semantics::ConcreteGraph as RustConcreteGraph;
    use ::grabapl::graph::operation::builder::{OperationBuilder as RustOperationBuilder, OperationBuilderError as RustOperationBuilderError};
    use simple_semantics::{BuiltinOperation, SimpleSemantics, BuiltinQuery as RustBuiltinQuery};
    use std::fmt::Write;

    use ::grabapl::graph::operation::user_defined::AbstractNodeId as RustAbstractNodeId;
    use grabapl::graph::operation::builder::Instruction;
    use grabapl::graph::operation::user_defined::AbstractOperationResultMarker;
    use grabapl::graph::pattern::AbstractOutputNodeMarker;
    use super::{
        RustEdgeAbstract, RustNodeAbstract, RustNodeConcrete, RustEdgeConcrete,
    };

    use grabapl::graph::operation::builder::IntermediateState as RustIntermediateState;


    #[diplomat::opaque]
    pub struct ConcreteGraph(RustConcreteGraph<SimpleSemantics>);
    #[diplomat::opaque]
    pub struct AbstractGraph(RustAbstractGraph<SimpleSemantics>);

    #[diplomat::opaque]
    pub struct DotCollector(grabapl::DotCollector);

    #[diplomat::opaque]
    pub struct OpCtx(grabapl::OperationContext<SimpleSemantics>);

    #[diplomat::opaque]
    pub struct OperationBuilder<'a>(RustOperationBuilder<'a, SimpleSemantics>);

    impl OpCtx {
        pub fn create() -> Box<OpCtx> {
            // TODO: define an init function that calls this
            console_error_panic_hook::set_once();
            log::error!("test log::error! call");
            let mut operation_ctx =
                ::grabapl::OperationContext::from_builtins(std::collections::HashMap::from([
                    (0, BuiltinOperation::AddNode),
                    (1, BuiltinOperation::AppendChild),
                    (2, BuiltinOperation::IndexCycle),
                    (
                        3,
                        BuiltinOperation::SetValue(Box::new(|| {
                            prompt("Set value:").parse().unwrap()
                        })),
                    ),
                ]));
            operation_ctx.add_custom_operation(5, simple_semantics::sample_user_defined_operations::get_labeled_edges_insert_bst_user_defined_operation(5));
            Box::new(OpCtx(operation_ctx))
        }
    }

    #[diplomat::opaque]
    pub struct Runner;

    impl Runner {
        pub fn create() -> Box<Runner> {
            Box::new(Runner)
        }

        pub fn run(&self, graph: &mut ConcreteGraph, op_ctx: &OpCtx, op_id: u32, args: &[u32]) {
            grabapl::graph::operation::run_from_concrete::<SimpleSemantics>(
                &mut graph.0,
                &op_ctx.0,
                op_id,
                args.to_vec(),
            )
            .unwrap();
        }
    }

    impl ConcreteGraph {
        pub fn create() -> Box<ConcreteGraph> {
            Box::new(ConcreteGraph(SimpleSemantics::new_concrete_graph()))
        }

        pub fn add_node(&mut self, value: i32) -> u32 {
            self.0.add_node(value)
        }

        pub fn add_edge(&mut self, from: u32, to: u32, value: &str) {
            self.0.add_edge(from, to, value.to_string());
        }

        // just for testing
        pub fn say_hi(&self) {
            let x = prompt("hi");
            if x == "panic" {
                panic!("test {}", x);
            }
            log::error!("doing thing {:?}", self.0.get_node_attr(0));
        }
    }

    impl DotCollector {
        pub fn create() -> Box<DotCollector> {
            Box::new(DotCollector(grabapl::DotCollector::new()))
        }

        pub fn collect(&mut self, graph: &ConcreteGraph) {
            self.0.collect(&graph.0);
        }

        pub fn get_dot(&self, dw: &mut DiplomatWrite) {
            write!(dw, "{}", self.0.finalize());
        }
    }

    impl<'a> OperationBuilder<'a> {
        // TODO: this is only safe as long as the OpCtx is not edited.
        pub fn create(op_ctx: &'a OpCtx) -> Box<OperationBuilder<'a>> {
            Box::new(OperationBuilder(RustOperationBuilder::new(&op_ctx.0)))
        }

        pub fn expect_parameter_node(&mut self, marker: u32) -> Result<(), Box<OperationBuilderError>> {
            self.0.expect_parameter_node(marker, ()).map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_context_node(&mut self, marker: u32) -> Result<(), Box<OperationBuilderError>> {
            self.0.expect_context_node(marker, ()).map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_parameter_edge(&mut self, src: u32, dst: u32, av: &EdgeAbstract) -> Result<(), Box<OperationBuilderError>> {
            self.0.expect_parameter_edge(src, dst, av.0.clone()).map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn start_query(&mut self, query: &BuiltinQuery, args: &AbstractArgList) -> Result<(), Box<OperationBuilderError>> {
            self.0.start_query(query.0.clone(), args.0.clone()).map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn enter_true_branch(&mut self) -> Result<(), Box<OperationBuilderError>> {
            self.0.enter_true_branch().map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn enter_false_branch(&mut self) -> Result<(), Box<OperationBuilderError>> {
            self.0.enter_false_branch().map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn start_shape_query(&mut self, query_name: &str) -> Result<(), Box<OperationBuilderError>> {
            // TODO: make the marker non-copy and owned
            let leaked = query_name.to_string().leak();
            let marker = AbstractOperationResultMarker::Custom(leaked);
            self.0.start_shape_query(marker).map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn end_query(&mut self) -> Result<(), Box<OperationBuilderError>> {
            self.0.end_query().map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_shape_node(&mut self, node_name: &str) -> Result<(), Box<OperationBuilderError>> {
            // TODO: make the marker non-copy and owned
            let node_marker = AbstractOutputNodeMarker(node_name.to_string().leak());
            self.0.expect_shape_node(node_marker, ()).map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_shape_edge(&mut self, src: &AbstractNodeId, dst: &AbstractNodeId, av: &EdgeAbstract) -> Result<(), Box<OperationBuilderError>> {
            self.0.expect_shape_edge(src.0.clone(), dst.0.clone(), av.0.clone()).map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn add_instruction(&mut self, name: Option<&str>, instruction: &mut BuilderOpLike, args: &AbstractArgList) -> Result<(), Box<OperationBuilderError>> {
            let instruction = instruction.0.take().expect("internal error: instruction missing");
            let args = args.0.clone();
            match name {
                Some(name) => {
                    let marker = AbstractOperationResultMarker::Custom(name.to_string().leak());
                    self.0.add_named_instruction(marker, instruction, args).map_err(|e| Box::new(OperationBuilderError(e)))
                }
                None => {
                    self.0.add_instruction(instruction, args).map_err(|e| Box::new(OperationBuilderError(e)))

                }
            }
        }

        pub fn show(&self) -> Result<Box<IntermediateState>, Box<OperationBuilderError>> {
            self.0.show_state().map(|s| Box::new(IntermediateState(s))).map_err(|e| Box::new(OperationBuilderError(e)))
        }

    }

    #[diplomat::opaque]
    pub struct IntermediateState(RustIntermediateState<SimpleSemantics>);

    impl IntermediateState {
        pub fn get_dot(&self, dw: &mut DiplomatWrite) {
            write!(dw, "{}", self.0.graph.dot()).unwrap();
        }

        pub fn available_aids(&self, dw: &mut DiplomatWrite) {
            let aids: Vec<RustAbstractNodeId> = self.0.node_keys_to_aid.right_values().cloned().collect();
            write!(dw, "Available AIDs: {:?}", aids).unwrap();
        }
    }

    // NOTE: The Option<> to take ownership of the inner value. Because cloning an operation may be difficult.
    #[diplomat::opaque]
    pub struct BuilderOpLike(Option<Instruction<SimpleSemantics>>);

    impl BuilderOpLike {
        pub fn new_from_id(op_id: u32) -> Box<BuilderOpLike> {
            Box::new(BuilderOpLike(Some(Instruction::FromOperationId(op_id))))
        }
    }

    #[diplomat::opaque]
    pub struct BuiltinQuery(RustBuiltinQuery);

    impl BuiltinQuery {
        pub fn new_is_value_gt(value: i32) -> Box<BuiltinQuery> {
            Box::new(BuiltinQuery(RustBuiltinQuery::IsValueGt(value)))
        }

        pub fn new_is_value_eq(value: i32) -> Box<BuiltinQuery> {
            Box::new(BuiltinQuery(RustBuiltinQuery::IsValueEq(value)))
        }

        pub fn new_values_equal() -> Box<BuiltinQuery> {
            Box::new(BuiltinQuery(RustBuiltinQuery::ValuesEqual))
        }

        pub fn new_first_gt_snd() -> Box<BuiltinQuery> {
            Box::new(BuiltinQuery(RustBuiltinQuery::FirstGtSnd))
        }
    }


    #[diplomat::opaque]
    pub struct AbstractNodeId(RustAbstractNodeId);

    impl AbstractNodeId {
        pub fn new_parameter(marker: u32) -> Box<AbstractNodeId> {
            Box::new(AbstractNodeId(RustAbstractNodeId::ParameterMarker(marker)))
        }

        pub fn new_from_output(op_marker: &str, node_marker: &str) -> Box<AbstractNodeId> {
            let aid = RustAbstractNodeId::DynamicOutputMarker(
                (&*node_marker.to_string().leak()).into(),
                (&*op_marker.to_string().leak()).into(),
            );
            Box::new(AbstractNodeId(aid))
        }
    }

    #[diplomat::opaque]
    pub struct AbstractArgList(Vec<RustAbstractNodeId>);

    impl AbstractArgList {
        pub fn create() -> Box<AbstractArgList> {
            Box::new(AbstractArgList(Vec::new()))
        }

        pub fn push(&mut self, arg: &AbstractNodeId) {
            self.0.push(arg.0.clone());
        }
    }

    #[diplomat::opaque]
    pub struct OperationBuilderError(RustOperationBuilderError);

    impl OperationBuilderError {
        pub fn message(&self, dw: &mut DiplomatWrite) {
            write!(dw, "{}", self.0).unwrap();
        }
    }

    #[diplomat::opaque]
    pub struct EdgeAbstract(RustEdgeAbstract);

    impl EdgeAbstract {
        pub fn new_wildcard() -> Box<EdgeAbstract> {
            Box::new(EdgeAbstract(RustEdgeAbstract::Wildcard))
        }

        pub fn new_exact(exact: &str) -> Box<EdgeAbstract> {
            Box::new(EdgeAbstract(RustEdgeAbstract::Exact(exact.to_string())))
        }
    }

}

use grabapl::Semantics;
use simple_semantics::SimpleSemantics;
use wasm_bindgen::prelude::wasm_bindgen;

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
    use ::grabapl::Semantics;
    use ::grabapl::graph::operation::builder::{
        OperationBuilder as RustOperationBuilder,
        OperationBuilderError as RustOperationBuilderError,
    };
    use ::grabapl::graph::semantics::AbstractGraph as RustAbstractGraph;
    use ::grabapl::graph::semantics::ConcreteGraph as RustConcreteGraph;
    use simple_semantics::{BuiltinOperation, BuiltinQuery as RustBuiltinQuery, SimpleSemantics};
    use std::fmt::Write;

    use super::{RustEdgeAbstract, RustEdgeConcrete, RustNodeAbstract, RustNodeConcrete};
    use ::grabapl::graph::operation::user_defined::AbstractNodeId as RustAbstractNodeId;
    use grabapl::graph::operation::builder::BuilderOpLike as RustBuilderOpLike;
    use grabapl::graph::operation::user_defined::AbstractOperationResultMarker;
    use grabapl::graph::pattern::AbstractOutputNodeMarker;

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
            operation_ctx.add_builtin_operation(6, BuiltinOperation::DeleteNode);
            Box::new(OpCtx(operation_ctx))
        }

        // TODO: dangerous function because it needs a mutable OpCtx while at the same time we store a reference
        //  to OpCtx in the OperationBuilder.
        pub fn add_custom_operation(&mut self, op_id: u32, operation: &mut UserDefinedOperation) {
            self.0
                .add_custom_operation(op_id, operation.0.take().unwrap());
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
        // TODO: this is only safe and sound as long as the OpCtx is not edited.
        pub fn create(op_ctx: &'a OpCtx) -> Box<OperationBuilder<'a>> {
            Box::new(OperationBuilder(RustOperationBuilder::new(&op_ctx.0)))
        }

        pub fn expect_parameter_node(
            &mut self,
            marker: u32,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .expect_parameter_node(marker, ())
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_context_node(
            &mut self,
            marker: u32,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .expect_context_node(marker, ())
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_parameter_edge(
            &mut self,
            src: u32,
            dst: u32,
            av: &EdgeAbstract,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .expect_parameter_edge(src, dst, av.0.clone())
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn start_query(
            &mut self,
            query: &BuiltinQuery,
            args: &AbstractArgList,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .start_query(query.0.clone(), args.0.clone())
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn enter_true_branch(&mut self) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .enter_true_branch()
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn enter_false_branch(&mut self) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .enter_false_branch()
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn start_shape_query(
            &mut self,
            query_name: &str,
        ) -> Result<(), Box<OperationBuilderError>> {
            // TODO: make the marker non-copy and owned
            let leaked = query_name.to_string().leak();
            let marker = AbstractOperationResultMarker::Custom(leaked);
            self.0
                .start_shape_query(marker)
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn end_query(&mut self) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .end_query()
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_shape_node(
            &mut self,
            node_name: &str,
        ) -> Result<(), Box<OperationBuilderError>> {
            // TODO: make the marker non-copy and owned
            let node_marker = AbstractOutputNodeMarker(node_name.to_string().leak());
            self.0
                .expect_shape_node(node_marker, ())
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_shape_edge(
            &mut self,
            src: &AbstractNodeId,
            dst: &AbstractNodeId,
            av: &EdgeAbstract,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .expect_shape_edge(src.0.clone(), dst.0.clone(), av.0.clone())
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn add_operation(
            &mut self,
            name: Option<&str>,
            instruction: &mut BuilderOpLike,
            args: &AbstractArgList,
        ) -> Result<(), Box<OperationBuilderError>> {
            let instruction = instruction
                .0
                .take()
                .expect("internal error: instruction missing");
            let args = args.0.clone();
            match name {
                Some(name) => {
                    let marker = AbstractOperationResultMarker::Custom(name.to_string().leak());
                    self.0
                        .add_named_operation(marker, instruction, args)
                        .map_err(|e| Box::new(OperationBuilderError(e)))
                }
                None => self
                    .0
                    .add_operation(instruction, args)
                    .map_err(|e| Box::new(OperationBuilderError(e))),
            }
        }

        pub fn show(&self) -> Result<Box<IntermediateState>, Box<OperationBuilderError>> {
            self.0
                .show_state()
                .map(|s| Box::new(IntermediateState(s)))
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn finalize(
            &self,
            op_id: u32,
        ) -> Result<Box<UserDefinedOperation>, Box<OperationBuilderError>> {
            self.0
                .build(op_id)
                .map(|op| Box::new(UserDefinedOperation(Some(op))))
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }
    }

    // Option again because cloning is difficult so we want to take.
    #[diplomat::opaque]
    pub struct UserDefinedOperation(
        Option<grabapl::graph::operation::user_defined::UserDefinedOperation<SimpleSemantics>>,
    );

    #[diplomat::opaque]
    pub struct IntermediateState(RustIntermediateState<SimpleSemantics>);

    impl IntermediateState {
        pub fn get_dot(&self, dw: &mut DiplomatWrite) {
            write!(dw, "{}", self.0.dot_with_aid()).unwrap();
        }

        pub fn available_aids(&self, dw: &mut DiplomatWrite) {
            // TODO: sort this to have a stable debug output
            let aids: Vec<RustAbstractNodeId> =
                self.0.node_keys_to_aid.right_values().cloned().collect();
            write!(dw, "Available AIDs: {:#?}", aids).unwrap();
        }

        pub fn query_context(&self, dw: &mut DiplomatWrite) {
            let last = self.0.query_path.last();
            if let Some(grabapl::graph::operation::builder::QueryPath::Query(name)) = last {
                write!(
                    dw,
                    "Currently designing query: {}. Enter true or false branch to proceed.\n",
                    name
                )
                .unwrap();
            }
            write!(dw, "Entire path: {:#?}", self.0.query_path).unwrap();
        }
    }

    // NOTE: The Option<> to take ownership of the inner value. Because cloning an operation may be difficult.
    #[diplomat::opaque]
    pub struct BuilderOpLike(Option<RustBuilderOpLike<SimpleSemantics>>);

    impl BuilderOpLike {
        pub fn new_from_id(op_id: u32) -> Box<BuilderOpLike> {
            Box::new(BuilderOpLike(Some(RustBuilderOpLike::FromOperationId(op_id))))
        }

        pub fn new_recurse() -> Box<BuilderOpLike> {
            Box::new(BuilderOpLike(Some(RustBuilderOpLike::Recurse)))
        }

        pub fn new_add_node() -> Box<BuilderOpLike> {
            Box::new(BuilderOpLike(Some(RustBuilderOpLike::Builtin(
                BuiltinOperation::AddNode,
            ))))
        }

        pub fn new_add_edge() -> Box<BuilderOpLike> {
            Box::new(BuilderOpLike(Some(RustBuilderOpLike::Builtin(
                BuiltinOperation::AddEdge,
            ))))
        }

        pub fn new_set_edge_value(value: &str) -> Box<BuilderOpLike> {
            Box::new(BuilderOpLike(Some(RustBuilderOpLike::Builtin(
                BuiltinOperation::SetEdgeValue(value.to_string()),
            ))))
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

        pub fn new_from_str(aid: &str) -> Box<AbstractNodeId> {
            // P() is param, O(op_marker, node_marker) is output
            let aid = if aid.starts_with("P(") {
                RustAbstractNodeId::ParameterMarker(aid[2..aid.len() - 1].parse().unwrap())
            } else if aid.starts_with("O(") {
                let parts: Vec<&str> = aid[2..aid.len() - 1].split(',').collect();
                RustAbstractNodeId::DynamicOutputMarker(
                    (&*parts[0].trim().to_string().leak()).into(),
                    (&*parts[1].trim().to_string().leak()).into(),
                )
            } else {
                panic!("Invalid AbstractNodeId format: {}", aid);
            };
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
        #[diplomat::attr(auto, stringifier)]
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

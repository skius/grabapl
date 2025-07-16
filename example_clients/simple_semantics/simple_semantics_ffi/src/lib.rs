use grabapl::Semantics;
use grabapl::graph::dot::DotCollector;
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

#[diplomat::bridge]
mod ffi {
    use super::prompt;
    use ::grabapl::Semantics;
    use error_stack::Report;
    use grabapl::operation::builder::{
        OperationBuilder as RustOperationBuilder,
        OperationBuilderError as RustOperationBuilderError,
    };
    use grabapl::semantics::AbstractGraph as RustAbstractGraph;
    use grabapl::semantics::ConcreteGraph as RustConcreteGraph;
    use simple_semantics::{BuiltinOperation, BuiltinQuery as RustBuiltinQuery, SimpleSemantics};
    use std::fmt::Write;
    use std::str::FromStr;

    use super::DotCollector as RustDotCollector;
    use super::RustEdgeAbstract;
    use grabapl::operation::builder::BuilderOpLike as RustBuilderOpLike;

    use grabapl::operation::user_defined::AbstractNodeId as RustAbstractNodeId;

    use grabapl::operation::builder::IntermediateState as RustIntermediateState;

    #[diplomat::opaque]
    pub struct ConcreteGraph(RustConcreteGraph<SimpleSemantics>);
    #[diplomat::opaque]
    pub struct AbstractGraph(RustAbstractGraph<SimpleSemantics>);

    #[diplomat::opaque]
    pub struct DotCollector(RustDotCollector);

    #[diplomat::opaque]
    pub struct OpCtx(grabapl::prelude::OperationContext<SimpleSemantics>);

    #[diplomat::opaque]
    pub struct OperationBuilder<'a>(RustOperationBuilder<'a, SimpleSemantics>);

    impl OpCtx {
        pub fn create() -> Box<OpCtx> {
            // TODO: define an init function that calls this
            console_error_panic_hook::set_once();
            log::error!("test log::error! call");
            let mut operation_ctx = ::grabapl::prelude::OperationContext::from_builtins(
                std::collections::HashMap::from([
                    (0, BuiltinOperation::AddNode),
                    (1, BuiltinOperation::AppendChild),
                    (2, BuiltinOperation::IndexCycle),
                    (
                        3,
                        BuiltinOperation::SetValue(Box::new(|| {
                            prompt("Set value:").parse().unwrap()
                        })),
                    ),
                ]),
            );
            operation_ctx.add_custom_operation(5, simple_semantics::sample_user_defined_operations::get_labeled_edges_insert_bst_user_defined_operation(&operation_ctx, 5));
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
            grabapl::operation::run_from_concrete::<SimpleSemantics>(
                &mut graph.0,
                &op_ctx.0,
                op_id,
                &args.iter().copied().map(Into::into).collect::<Vec<_>>(),
            )
            .unwrap();
        }
    }

    impl ConcreteGraph {
        pub fn create() -> Box<ConcreteGraph> {
            Box::new(ConcreteGraph(SimpleSemantics::new_concrete_graph()))
        }

        pub fn add_node(&mut self, value: i32) -> u32 {
            self.0.add_node(value).0
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
            log::error!("doing thing {:?}", self.0.get_node_attr(0.into()));
        }
    }

    impl DotCollector {
        pub fn create() -> Box<DotCollector> {
            Box::new(DotCollector(RustDotCollector::new()))
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
        pub fn create(op_ctx: &'a OpCtx, self_op_id: u32) -> Box<OperationBuilder<'a>> {
            Box::new(OperationBuilder(RustOperationBuilder::new(
                &op_ctx.0, self_op_id,
            )))
        }

        pub fn expect_parameter_node(
            &mut self,
            marker: &str,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .expect_parameter_node(marker, ())
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_context_node(
            &mut self,
            marker: &str,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .expect_context_node(marker, ())
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn expect_parameter_edge(
            &mut self,
            src: &str,
            dst: &str,
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
            op_marker: &str,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .start_shape_query(op_marker)
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
            let node_marker = node_name.into();
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
                    let marker = name.into();
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

        pub fn rename_node(
            &mut self,
            aid: &AbstractNodeId,
            new_name: &str,
        ) -> Result<(), Box<OperationBuilderError>> {
            self.0
                .rename_node(aid.0.clone(), new_name)
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn show(&self) -> Result<Box<IntermediateState>, Box<OperationBuilderError>> {
            self.0
                .show_state()
                .map(|s| Box::new(IntermediateState(s)))
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }

        pub fn finalize(
            &mut self,
        ) -> Result<Box<UserDefinedOperation>, Box<OperationBuilderError>> {
            self.0
                .build()
                .map(|op| Box::new(UserDefinedOperation(Some(op))))
                .map_err(|e| Box::new(OperationBuilderError(e)))
        }
    }

    // Option again because cloning is difficult so we want to take.
    #[diplomat::opaque]
    pub struct UserDefinedOperation(
        Option<grabapl::operation::user_defined::UserDefinedOperation<SimpleSemantics>>,
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
            if let Some(grabapl::operation::builder::QueryPath::Query(name)) = last {
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
            Box::new(BuilderOpLike(Some(RustBuilderOpLike::FromOperationId(
                op_id,
            ))))
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
        pub fn new_parameter(marker: &str) -> Box<AbstractNodeId> {
            Box::new(AbstractNodeId(RustAbstractNodeId::param(marker)))
        }

        pub fn new_from_output(op_marker: &str, node_marker: &str) -> Box<AbstractNodeId> {
            let aid = RustAbstractNodeId::dynamic_output(op_marker, node_marker);
            Box::new(AbstractNodeId(aid))
        }

        pub fn new_from_str(aid: &str) -> Box<AbstractNodeId> {
            Box::new(AbstractNodeId(RustAbstractNodeId::from_str(aid).unwrap()))
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
    pub struct OperationBuilderError(Report<RustOperationBuilderError>);

    impl OperationBuilderError {
        #[diplomat::attr(auto, stringifier)]
        pub fn message(&mut self, dw: &mut DiplomatWrite) {
            Report::set_color_mode(error_stack::fmt::ColorMode::None);
            write!(dw, "{:?}", self.0).unwrap();
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

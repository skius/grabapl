use grabapl_template_semantics::{EdgeType, EdgeValue, NodeType, NodeValue, TheSemantics};
use syntax::custom_syntax::CustomSyntax;
use syntax::interpreter::lex_then_parse;

fn parse_node_value(s: &str) -> Option<NodeValue> {
    let parser = grabapl_template_semantics::syntax::node_value_parser();
    lex_then_parse(s, parser).ok()
}

fn parse_edge_value(s: &str) -> Option<EdgeValue> {
    let parser = grabapl_template_semantics::syntax::edge_value_parser();
    lex_then_parse(s, parser).ok()
}

fn parse_node_type(s: &str) -> Result<NodeType, String> {
    let parser = grabapl_template_semantics::syntax::TheCustomSyntax::get_node_type_parser();
    lex_then_parse(s, parser).map_err(|e| e.to_string())
}

fn parse_edge_type(s: &str) -> Result<EdgeType, String> {
    let parser = grabapl_template_semantics::syntax::TheCustomSyntax::get_edge_type_parser();
    lex_then_parse(s, parser).map_err(|e| e.to_string())
}

fn node_value_to_string(value: &NodeValue) -> String {
    match value {
        NodeValue::Integer(x) => x.to_string(),
        NodeValue::String(x) => {
            // debug, since we want surrounding quotes
            format!("{x:?}")
        }
    }
}

fn edge_value_to_string(value: &EdgeValue) -> String {
    match value {
        EdgeValue::Unit => "()".to_string(),
        EdgeValue::String(x) => {
            // debug, since we want surrounding quotes
            format!("{x:?}")
        }
        EdgeValue::Integer(x) => x.to_string(),
    }
}

#[diplomat::bridge]
pub mod ffi {
    use super::NodeValue;
    use grabapl::NodeKey;
    use grabapl::graph::GraphTrait;
    use grabapl::prelude::{AbstractNodeId, OperationId};
    use grabapl_template_semantics::EdgeValue;
    use std::collections::HashMap;
    use std::fmt::Write;
    use std::result::Result;
    use grabapl::operation::user_defined::AbstractOperationResultMarker;
    use syntax::WithLineColSpans;

    pub struct Context {
        pub i: i32,
    }

    impl Context {
        pub fn init() {
            console_error_panic_hook::set_once();
        }

        pub fn parse(src: &str) -> Box<ParseResult> {
            let res = syntax::try_parse_to_op_ctx_and_map::<super::TheSemantics>(src, true);

            let inner_res = match res.op_ctx_and_map {
                Ok((op_ctx, fn_names)) => {
                    let op_ctx_and_fn_names = OpCtxAndFnNames {
                        op_ctx,
                        fn_names: fn_names
                            .into_iter()
                            .map(|(k, v)| (k.to_string(), v))
                            .collect(),
                    };
                    Ok(op_ctx_and_fn_names)
                }
                Err(e) => Err(e),
            };

            Box::new(ParseResult {
                result: inner_res,
                state_map: res.state_map,
            })
        }
    }

    #[diplomat::opaque]
    struct OpCtxAndFnNames {
        op_ctx: grabapl::operation::OperationContext<super::TheSemantics>,
        fn_names: HashMap<String, OperationId>,
    }

    #[diplomat::opaque]
    pub struct ParseResult {
        result: Result<OpCtxAndFnNames, WithLineColSpans<String>>,
        state_map:
            HashMap<String, grabapl::operation::builder::IntermediateState<super::TheSemantics>>,
    }

    impl ParseResult {
        /// Writes the error message if one exists.
        pub fn error_message(&self, out: &mut DiplomatWrite) {
            if let Err(ref e) = self.result {
                write!(out, "{}", e.value).unwrap();
            }
        }

        /// Returns an interable of error spans (if any).
        pub fn error_spans(&self) -> Box<LineColSpansIter> {
            let spans = match &self.result {
                Ok(_) => {
                    vec![]
                }
                Err(with_line_col_spans) => with_line_col_spans
                    .spans
                    .iter()
                    .map(|span| LineColSpan {
                        line_start: span.line_start,
                        line_end: span.line_end,
                        col_start: span.col_start,
                        col_end: span.col_end,
                    })
                    .collect::<Vec<_>>(),
            };
            Box::new(LineColSpansIter(spans.into_iter()))
        }

        /// Returns the DOT representation of the intermediate state named `state`.
        pub fn dot_of_state(&self, state: &str, dot: &mut DiplomatWrite) {
            let Some(state) = self.state_map.get(state) else {
                log::error!("state does not exist in state map");
                return;
            };
            write!(dot, "{}", state.dot_with_aid_with_dot_syntax()).unwrap();
        }

        /// Lists the available states.
        pub fn list_states(&self) -> Box<StringIter> {
            let mut states: Vec<String> = self.state_map.keys().cloned().collect();
            states.sort_unstable();
            Box::new(StringIter(states.into_iter()))
        }

        /// Lists the available operations.
        pub fn list_operations(&self) -> Box<StringIter> {
            let mut operations: Vec<String> = self
                .result
                .as_ref()
                .map(|res| res.fn_names.keys().cloned().collect())
                .unwrap_or_default();
            operations.sort_unstable();
            Box::new(StringIter(operations.into_iter()))
        }

        /// Runs the operation with the given name and arguments on the provided concrete graph.
        pub fn run_operation(
            &self,
            g: &mut ConcreteGraph,
            op_name: &str,
            args: &[u32],
        ) -> Result<Box<NewNodesIter>, Box<StringError>> {
            let op_ctx = self
                .result
                .as_ref()
                .map_err(|e| Box::new(StringError(e.value.clone())))?;
            let op_id = op_ctx.fn_names.get(op_name).ok_or_else(|| {
                Box::new(StringError(format!("Operation '{}' not found", op_name)))
            })?;

            let res = grabapl::prelude::run_from_concrete(
                &mut g.graph,
                &op_ctx.op_ctx,
                *op_id,
                &args.iter().copied().map(NodeKey).collect::<Vec<_>>(),
            );
            match res {
                Ok(output) => Ok(Box::new(NewNodesIter(
                    output
                        .new_nodes()
                        .into_iter()
                        .map(|(&marker, &key)| {
                            (
                                key.0,
                                marker.0.to_string(),
                                super::node_value_to_string(g.graph.get_node_attr(key).unwrap()),
                            )
                        })
                        .collect::<Vec<_>>()
                        .into_iter(),
                ))),
                Err(e) => Err(Box::new(StringError(e.to_string()))),
            }
        }
    }

    #[diplomat::opaque]
    pub struct StringIter(std::vec::IntoIter<String>);

    impl StringIter {
        #[diplomat::attr(auto, iterator)]
        pub fn next(&mut self) -> Option<Box<StringWrapper>> {
            self.0.next().map(|s| Box::new(StringWrapper(s)))
        }

        #[diplomat::attr(auto, iterable)]
        pub fn to_iterable(&self) -> Box<StringIter> {
            Box::new(StringIter(self.0.clone()))
        }
    }

    #[diplomat::opaque]
    pub struct StringWrapper(String);

    impl StringWrapper {
        pub fn new(s: &str) -> Box<Self> {
            Box::new(StringWrapper(s.to_string()))
        }

        #[diplomat::attr(auto, stringifier)]
        pub fn to_string(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.0).unwrap();
        }
    }

    #[diplomat::opaque]
    pub struct StringError(String);

    impl StringError {
        pub fn to_string(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.0).unwrap();
        }
    }

    // === Concrete Graph and Execution ===
    #[diplomat::opaque]
    pub struct ConcreteGraph {
        graph: grabapl::prelude::ConcreteGraph<super::TheSemantics>,
    }

    impl ConcreteGraph {
        pub fn new() -> Box<Self> {
            Box::new(ConcreteGraph {
                graph: grabapl::prelude::ConcreteGraph::<super::TheSemantics>::new(),
            })
        }

        /// Returns the node key of the newly added node
        pub fn add_node(&mut self, value: &str) -> u32 {
            let value = super::parse_node_value(value)
                // if we failed a parse, assume it's just a string
                .unwrap_or(NodeValue::String(value.to_string()));
            self.graph.add_node(value).0
        }

        pub fn delete_node(&mut self, key: u32) {
            self.graph.delete_node(NodeKey(key));
        }

        pub fn add_edge(&mut self, src: u32, dst: u32, weight: &str) {
            self.graph.add_edge(
                src,
                dst,
                super::parse_edge_value(weight)
                    // if we failed a parse, assume it's just a string
                    .unwrap_or(EdgeValue::String(weight.to_string())),
            );
        }

        pub fn get_nodes(&self) -> Box<NodesIter> {
            let nodes: Vec<(u32, String)> = self
                .graph
                .nodes()
                .map(|(k, v)| (k.0, super::node_value_to_string(v)))
                .collect();
            Box::new(NodesIter(nodes.into_iter()))
        }

        pub fn get_edges(&self) -> Box<EdgesIter> {
            let edges: Vec<(u32, u32, String)> = self
                .graph
                .edges()
                .map(|(src, dst, weight)| (src.0, dst.0, super::edge_value_to_string(weight)))
                .collect();
            Box::new(EdgesIter(edges.into_iter()))
        }

        // pub fn dot(&self) -> String {
        //     self.graph.dot()
        // }
    }

    #[diplomat::opaque]
    pub struct NodesIter(std::vec::IntoIter<(u32, String)>);

    impl NodesIter {
        #[diplomat::attr(auto, iterator)]
        pub fn next(&mut self) -> Option<Box<NodeWrapper>> {
            self.0
                .next()
                .map(|(k, v)| Box::new(NodeWrapper { key: k, value: v }))
        }

        #[diplomat::attr(auto, iterable)]
        pub fn to_iterable(&self) -> Box<NodesIter> {
            Box::new(NodesIter(self.0.clone()))
        }
    }

    #[diplomat::opaque]
    pub struct NodeWrapper {
        key: u32,
        value: String,
    }

    impl NodeWrapper {
        pub fn key(&self) -> u32 {
            self.key
        }

        pub fn value(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.value).unwrap();
        }

        // #[diplomat::attr(auto, stringifier)]
        // pub fn to_string(&self, out: &mut DiplomatWrite) {
        //     write!(out, "Node(key: {}, value: {})", self.key, self.value).unwrap();
        // }
    }

    #[diplomat::opaque]
    pub struct EdgesIter(std::vec::IntoIter<(u32, u32, String)>);

    impl EdgesIter {
        #[diplomat::attr(auto, iterator)]
        pub fn next(&mut self) -> Option<Box<EdgeWrapper>> {
            self.0
                .next()
                .map(|(src, dst, weight)| Box::new(EdgeWrapper { src, dst, weight }))
        }

        #[diplomat::attr(auto, iterable)]
        pub fn to_iterable(&self) -> Box<EdgesIter> {
            Box::new(EdgesIter(self.0.clone()))
        }
    }

    #[diplomat::opaque]
    pub struct EdgeWrapper {
        src: u32,
        dst: u32,
        weight: String,
    }

    impl EdgeWrapper {
        pub fn src(&self) -> u32 {
            self.src
        }

        pub fn dst(&self) -> u32 {
            self.dst
        }

        pub fn weight(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.weight).unwrap();
        }
    }

    #[diplomat::opaque]
    pub struct NewNodesIter(std::vec::IntoIter<(u32, String, String)>);

    impl NewNodesIter {
        #[diplomat::attr(auto, iterator)]
        pub fn next(&mut self) -> Option<Box<NewNode>> {
            self.0.next().map(|(k, name, value)| {
                Box::new(NewNode {
                    key: k,
                    name,
                    value,
                })
            })
        }

        #[diplomat::attr(auto, iterable)]
        pub fn to_iterable(&self) -> Box<NewNodesIter> {
            Box::new(NewNodesIter(self.0.clone()))
        }
    }

    #[diplomat::opaque]
    pub struct NewNode {
        key: u32,
        name: String,
        value: String,
    }

    impl NewNode {
        pub fn key(&self) -> u32 {
            self.key
        }

        pub fn name(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.name).unwrap();
        }

        pub fn value(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.value).unwrap();
        }
    }

    #[diplomat::opaque]
    pub struct LineColSpansIter(std::vec::IntoIter<LineColSpan>);

    impl LineColSpansIter {
        #[diplomat::attr(auto, iterator)]
        pub fn next(&mut self) -> Option<LineColSpan> {
            self.0.next().map(|span| LineColSpan {
                line_start: span.line_start,
                line_end: span.line_end,
                col_start: span.col_start,
                col_end: span.col_end,
            })
        }

        #[diplomat::attr(auto, iterable)]
        pub fn to_iterable(&self) -> Box<LineColSpansIter> {
            Box::new(LineColSpansIter(self.0.clone()))
        }
    }

    #[derive(Clone)]
    pub struct LineColSpan {
        pub line_start: usize,
        pub line_end: usize,
        pub col_start: usize,
        pub col_end: usize,
    }
}

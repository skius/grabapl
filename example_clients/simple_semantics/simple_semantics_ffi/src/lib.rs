use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    fn prompt(s: &str) -> String;
}

#[diplomat::bridge]
mod ffi {
    use grabapl::graph::semantics::AbstractGraph as RustAbstractGraph;
    use grabapl::graph::semantics::ConcreteGraph as RustConcreteGraph;
    use grabapl::Semantics;
    use simple_semantics::{BuiltinOperation, SimpleSemantics};
    use super::prompt;
    use std::fmt::Write;

    #[diplomat::opaque]
    pub struct ConcreteGraph(RustConcreteGraph<SimpleSemantics>);
    #[diplomat::opaque]
    pub struct AbstractGraph(RustAbstractGraph<SimpleSemantics>);

    #[diplomat::opaque]
    pub struct DotCollector(grabapl::DotCollector);

    #[diplomat::opaque]
    pub struct OpCtx(grabapl::OperationContext<SimpleSemantics>);

    impl OpCtx {
        pub fn create() -> Box<OpCtx> {
            // TODO: define an init function that calls this
            console_error_panic_hook::set_once();
            log::error!("test log::error! call");
            let mut operation_ctx = grabapl::OperationContext::from_builtins(
                std::collections::HashMap::from([
                    (0, BuiltinOperation::AddNode),
                    (1, BuiltinOperation::AppendChild),
                    (2, BuiltinOperation::IndexCycle),
                    (3, BuiltinOperation::SetValue(Box::new(|| {
                        prompt("Set value:").parse().unwrap()
                    }))),
                ])
            );
            operation_ctx.add_custom_operation(5, simple_semantics::sample_user_defined_operations::get_labeled_edges_insert_bst_user_defined_operation(5));
            Box::new(OpCtx(operation_ctx))
        }
    }

    #[diplomat::opaque]
    pub struct Runner;

    pub type VecU32 = Vec<u32>;

    impl Runner {
        pub fn create() -> Box<Runner> {
            Box::new(Runner)
        }

        pub fn run(&self, graph: &mut ConcreteGraph, op_ctx: &OpCtx, op_id: u32, args: &[u32]) {
            grabapl::graph::operation::run_operation::<SimpleSemantics>(
                &mut graph.0,
                &op_ctx.0,
                op_id,
                args.to_vec()
            ).unwrap();
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


}
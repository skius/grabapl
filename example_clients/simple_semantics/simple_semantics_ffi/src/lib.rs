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
    use simple_semantics::SimpleSemantics;
    use super::prompt;
    use std::fmt::Write;

    #[diplomat::opaque]
    pub struct ConcreteGraph(RustConcreteGraph<SimpleSemantics>);
    #[diplomat::opaque]
    pub struct AbstractGraph(RustAbstractGraph<SimpleSemantics>);

    #[diplomat::opaque]
    pub struct DotCollector(grabapl::DotCollector);

    impl ConcreteGraph {
        pub fn create() -> Box<ConcreteGraph> {
            // TODO: have init function for panic hook
            console_error_panic_hook::set_once();
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
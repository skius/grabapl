use grabapl::Semantics;
use grabapl::prelude::run_from_concrete;
use grabapl::semantics::example::{ExampleSemantics as TestSemantics, NodeValue};

const SRC: &'static str = stringify!(
    fn force_child(
        p1: int
    ) -> (child: int) {
        if shape [child: int, p1 -> child: *] {
            // child exists in this scope
        } else {
            diverge<"no child found">();
            // child does not exist in this scope, but ît diverged
        }
        // hence the child node should be visible here

        return (child: child);
    }
);

#[test]
fn diverge_test() {
    let (op_ctx, fn_map) = syntax::parse_to_op_ctx_and_map::<TestSemantics>(SRC);

    // assert!(false);
}

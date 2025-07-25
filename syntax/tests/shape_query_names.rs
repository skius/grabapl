use grabapl::semantics::example::ExampleSemantics as TestSemantics;

const SRC: &'static str = stringify!(
    fn test(
        p1: int
    ) {
        if shape [child: int, p1 -> child: *] {
            // child exists in this scope
        } else {
            // create a child
            let! child = add_node<int, 5>();
            add_edge<"child">(p1, child);
            // child now also exists in this scope
        }
        // hence the two nodes should be merged into this scope

        // use 'child' in some way
        copy_value_from_to(p1, child);
    }
);

#[test]
fn shape_query_node_is_merged() {
    let (op_ctx, fn_map) = grabapl_syntax::parse_to_op_ctx_and_map::<TestSemantics>(SRC);

    // assert!(false);
}

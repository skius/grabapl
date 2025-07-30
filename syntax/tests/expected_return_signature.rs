use grabapl::Semantics;
use grabapl::prelude::run_from_concrete;
use grabapl::semantics::example::ExampleSemantics;

#[test_log::test]
fn expected_return_signature_works() {
    let (op_ctx, fn_names) = grabapl_syntax::grabapl_parse!(ExampleSemantics,

        // testing if unnamed binding hides nodes
        fn test_binding() {
            let! a = add_node<int, 0>();
            foo(a);
            // no binding => cannot call requires_two_children
            // but can we see the nodes dynamically?
            trace();
        }


        // test return signature
        fn client(a: int, b: int) {
            let res = foo(a);
            requires_two_children(a);
            let res2 = foo2(a);
            requires_two_children(a);
        }

        fn requires_two_children(a: int) [
            b: int, c: int, a -> b: *, a -> c: "child"
        ] {}

        fn foo(a: int)
        -> (b: int, c: int, a -> b: *, a -> c: "child")
        {
            let! b = add_node<int, 0>();
            let! c = add_node<int, 0>();
            add_edge<"hello">(a, b);
            add_edge<"child">(a, c);
            return (b: b, c: c, a -> b: *, a -> c: "child");
        }

        fn foo2(a: int)
        -> (b: int, c: int)
        {
            let! b = add_node<int, 0>();
            let! c = add_node<int, 0>();
            add_edge<"hello">(a, b);
            add_edge<"child">(a, c);
            // NOTE: specifying return signature above is optional for edges
            return (b: b, c: c, a -> b: *, a -> c: "child");
        }



    );

    let mut g = ExampleSemantics::new_concrete_graph();
    let op_id = fn_names["test_binding"];
    let mut res = run_from_concrete(&mut g, &op_ctx, op_id, &[]).unwrap();
    let frame = res.trace.frames.pop().unwrap();
    println!("Frame: {}", frame.dot());

    // assert!(false);
}

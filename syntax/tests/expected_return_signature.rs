use grabapl::semantics::example::ExampleSemantics;

#[test_log::test]
fn expected_return_signature_works() {
    let _ = grabapl_syntax::grabapl_parse!(ExampleSemantics,

        fn client(a: int) {
            let res = foo(a);
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
    );
}
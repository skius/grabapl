use grabapl::prelude::*;

#[diplomat::bridge]
pub mod ffi {
    pub struct Context {
        pub i: i32,
    }

    impl Context {
        pub fn init() {
            console_error_panic_hook::set_once();

        }

        pub fn parse(src: &str) {
            let (op_ctx, fn_names) = syntax::parse_to_op_ctx_and_map::<grabapl::semantics::example::ExampleSemantics>(src);
            log::info!("fn names: {:?}", fn_names);
        }
    }


}
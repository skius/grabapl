use grabapl::prelude::*;

#[diplomat::bridge]
pub mod ffi {
    use std::collections::HashMap;
    use std::fmt::Write;
    use grabapl::prelude::OperationId;

    pub struct Context {
        pub i: i32,
    }

    impl Context {
        pub fn init() {
            console_error_panic_hook::set_once();

        }

        pub fn parse(src: &str) -> Result<Box<ParseResult>, Box<ParseError>> {
            let res = syntax::try_parse_to_op_ctx_and_map::<grabapl::semantics::example::ExampleSemantics>(src, true);
            match res {
                Ok((op_ctx, fn_names)) => {
                    let parse_result = ParseResult {
                        op_ctx,
                        fn_names: fn_names.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
                    };
                    Ok(Box::new(parse_result))
                }
                Err(e) => {
                    let parse_error = ParseError(e.to_string());
                    Err(Box::new(parse_error))
                }
            }

        }
    }

    #[diplomat::opaque]
    pub struct ParseResult {
        op_ctx: grabapl::operation::OperationContext<grabapl::semantics::example::ExampleSemantics>,
        fn_names: HashMap<String, OperationId>,
    }

    #[diplomat::opaque]
    pub struct ParseError(String);

    impl ParseError {
        pub fn to_string(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.0).unwrap();
        }
    }


}
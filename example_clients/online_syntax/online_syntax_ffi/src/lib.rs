use grabapl::prelude::*;

#[diplomat::bridge]
pub mod ffi {
    use std::collections::HashMap;
    use std::result::Result;
    use std::fmt::Write;
    use grabapl::prelude::OperationId;

    pub struct Context {
        pub i: i32,
    }

    impl Context {
        pub fn init() {
            console_error_panic_hook::set_once();

        }

        pub fn parse(src: &str) -> Box<ParseResult> {
            let res = syntax::try_parse_to_op_ctx_and_map::<grabapl::semantics::example::ExampleSemantics>(src, true);

            let inner_res = match res.op_ctx_and_map {
                Ok((op_ctx, fn_names)) => {
                    let op_ctx_and_fn_names = OpCtxAndFnNames {
                        op_ctx,
                        fn_names: fn_names.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
                    };
                    Ok(op_ctx_and_fn_names)
                }
                Err(e) => {
                    Err(e)
                }
            };

            Box::new(ParseResult {
                result: inner_res,
                state_map: res.state_map,
            })
        }
    }

    #[diplomat::opaque]
    struct OpCtxAndFnNames {
        op_ctx: grabapl::operation::OperationContext<grabapl::semantics::example::ExampleSemantics>,
        fn_names: HashMap<String, OperationId>,
    }

    #[diplomat::opaque]
    pub struct ParseResult {
        result: Result<OpCtxAndFnNames, String>,
        state_map: HashMap<String, grabapl::operation::builder::IntermediateState<grabapl::semantics::example::ExampleSemantics>>,
    }

    impl ParseResult {
        pub fn error_message(&self, out: &mut DiplomatWrite) {
            if let Err(ref e) = self.result {
                write!(out, "{}", e).unwrap();
            }
        }

        pub fn dot_of_state(&self, state: &str, dot: &mut DiplomatWrite) {
            let Some(state) = self.state_map.get(state) else {
                log::error!("state does not exist in state map");
                return;
            };
            write!(dot, "{}", state.dot_with_aid()).unwrap();
        }

        pub fn list_states(&self) -> Box<StringIter> {
            let mut states: Vec<String> = self.state_map.keys().cloned().collect();
            states.sort_unstable();
            Box::new(StringIter(states.into_iter()))
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
    pub struct ParseError(String);

    impl ParseError {
        pub fn to_string(&self, out: &mut DiplomatWrite) {
            write!(out, "{}", self.0).unwrap();
        }
    }


}
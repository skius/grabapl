// use syntax_macro::grabapl_source;
//
// macro_rules! wrapped_grabapl_source {
//     ($($t:tt)*) => {
//         grabapl_source!(stringify!($($t)*))
//     };
// }
//
// #[test]
// fn test() {
//     let (test, x) = grabapl_source!(
//         fn hello() {
//
//         }
//     );
//
//     let op_ctx_json = serde_json::to_string_pretty(&test).unwrap();
//
//     // assert!(false, "{op_ctx_json}");
// }

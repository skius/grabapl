use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

pub fn my_proc_impl(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let _ = syntax::parse_to_op_ctx_and_map(&s); // just parsing to check for errors

    quote!({ syntax::parse_to_op_ctx_and_map(&#s) })
}
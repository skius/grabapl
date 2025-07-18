use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use quote::__private::ext::RepToTokensExt;
use syn::parse_macro_input;
use syn::spanned::Spanned;

pub fn my_proc_impl(input: TokenStream) -> TokenStream {
    // parse input stream like so:
    // my_proc_impl!(TypeName, ... the entire rest...);
    // need to get TypeName out.

    // oh :( cannot do compile time parsing anymore since we need generics.

    todo!()

    // let s = input.to_string();
    // let _ = syntax::parse_to_op_ctx_and_map(&s); // just parsing to check for errors
    //
    // quote!({ syntax::parse_to_op_ctx_and_map(&#s) })
}
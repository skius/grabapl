//! This FFI crate exposes any functionality of [`grabapl`] and the custom semantics to other languages.
//!
//! This example uses the [Diplomat] tool to automatically generate idiomatic FFI bindings to
//! multiple target languages.
//!
//! See the main `README.md` for information on how to build this crate and integrate it
//! into a different language project.
//!
//! [Diplomat]: https://github.com/rust-diplomat/diplomat/

use grabapl::prelude::*;

/// This module is sent to Diplomat to automatically generate FFI bindings from functions and types
/// on both the Rust and the target language side.
///
/// See [The Diplomat Book] for detailed information on how and which types and functions can be
/// exposed with Diplomat.
///
/// In general, we will create a `diplomat::opaque` wrapper type for every type we want to expose,
/// which must be created with a `Box<Self>` return type.
///
/// [The Diplomat Book]: https://rust-diplomat.github.io/diplomat/
#[diplomat::bridge]
mod ffi {
    // #[diplomat::opaque]
    // pub struct Grabapl(i32);
    //
    // impl Grabapl {
    //     pub fn create() -> Box<Grabapl> {
    //         Box::new(Grabapl(0))
    //     }
    // }
}
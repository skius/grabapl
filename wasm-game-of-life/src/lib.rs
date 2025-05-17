mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn prompt(s: &str) -> String;

}


#[diplomat::bridge]
mod ffi {
    use crate::{prompt};

    pub struct MyFFIStruct {
        pub a: i32,
        pub b: bool,
    }

    impl MyFFIStruct {
        pub fn create() -> MyFFIStruct {
            MyFFIStruct {
                a: 42,
                b: true
            }
        }

        pub fn do_a_thing(self) {

            let x = prompt("Doing a thing");
            // panic!("test {}", x);

            // TODO: Get logging/panicking to work. Prompt works so why the rest not?
            // panic gets "RuntimeError: unreachable executed"
            
            // it works now? no idea why.
            
            log::error!("doing thing {:?}, {x}", self.b);
        }
    }
}
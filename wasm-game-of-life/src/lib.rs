mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    let x: fn(&str) = alert;

    x("Hello, wasm-game-of-life!");
}


#[diplomat::bridge]
mod ffi {
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
            println!("doing thing {:?}", self.b);
        }
    }
}
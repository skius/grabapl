pub mod bimap;

#[macro_export]
macro_rules! interned_string_newtype {
    ($ty_name:ident, $mk_fn:expr) => {
        impl<'a> From<&'a str> for $ty_name {
            fn from(value: &'a str) -> Self {
                $mk_fn(Intern::from_ref(value))
            }
        }
        
        impl From<String> for $ty_name {
            fn from(value: String) -> Self {
                $mk_fn(value.into())
            }
        } 
    };
    ($ty_name:ident) => {
        interned_string_newtype!($ty_name, $ty_name);
    }
}
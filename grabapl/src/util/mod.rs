pub mod bimap;

// TODO: In general, consider if we really want a global interning pool for strings that is non-droppable (i.e., causes leaks).
//  I think it's fine, because it's essentially just for variable names.
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
    };
}
pub mod log {
    #[allow(unused)]
    macro_rules! trace { ($($x:tt)*) => (
        #[cfg(feature = "log")] {
            log_crate::trace!($($x)*)
        }
    ) }
    #[allow(unused)]
    macro_rules! debug { ($($x:tt)*) => (
        #[cfg(feature = "log")] {
            log_crate::debug!($($x)*)
        }
    ) }
    #[allow(unused)]
    macro_rules! info { ($($x:tt)*) => (
        #[cfg(feature = "log")] {
            log_crate::info!($($x)*);
        }
    ) }
    #[allow(unused)]
    macro_rules! warn2 { ($($x:tt)*) => (
        #[cfg(feature = "log")] {
            log_crate::warn!($($x)*)
        }
    ) }
    #[allow(unused)]
    macro_rules! error { ($($x:tt)*) => (
        #[cfg(feature = "log")] {
            log_crate::error!($($x)*)
        }
    ) }

    #[allow(unused)]
    pub(crate) use {trace, debug, info, error, warn2 as warn};
}

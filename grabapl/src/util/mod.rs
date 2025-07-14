use derive_more::{From, Into};
use internment::Intern;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::Deref;

pub mod bimap;

// newtype necessary for to Serialize/Deserialize support.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, From)]
pub struct InternString(Intern<String>);

impl Deref for InternString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for InternString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "serde")]
impl Serialize for InternString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for InternString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let interned_string: Intern<String> = s.into();
        Ok(InternString(interned_string))
    }
}

// TODO: In general, consider if we really want a global interning pool for strings that is non-droppable (i.e., causes leaks).
//  I think it's fine, because it's essentially just for variable names.
#[macro_export]
macro_rules! interned_string_newtype {
    ($ty_name:ident, $mk_fn:expr) => {
        impl<'a> From<&'a str> for $ty_name {
            fn from(value: &'a str) -> Self {
                $mk_fn(Intern::<String>::from_ref(value).into())
            }
        }

        impl From<String> for $ty_name {
            fn from(value: String) -> Self {
                $mk_fn(Intern::<String>::from(value).into())
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
    pub(crate) use {debug, error, info, trace, warn2 as warn};
}

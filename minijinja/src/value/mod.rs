//! Provides a dynamic value type abstraction.
//!
//! This module gives access to a dynamically typed value which is used by
//! the template engine during execution.
//!
//! For the most part the existence of the value type can be ignored as
//! MiniJinja will perform the necessary conversions for you.  For instance
//! if you write a filter that converts a string you can directly declare the
//! filter to take a [`String`](std::string::String).  However for some more
//! advanced use cases it's useful to know that this type exists.
//!
//! # Converting ValueBoxs
//!
//! ValueBoxs are typically created via the [`From`] trait:
//!
//! ```
//! # use minijinja::value::ValueBox;
//! let int_value = ValueBox::from(42);
//! let none_value = ValueBox::from(());
//! let true_value = ValueBox::from(true);
//! ```
//!
//! Or via the [`FromIterator`] trait:
//!
//! ```
//! # use minijinja::value::ValueBox;
//! // collection into a sequence
//! let value: ValueBox = (1..10).into_iter().collect();
//!
//! // collection into a map
//! let value: ValueBox = [("key", "value")].into_iter().collect();
//! ```
//!
//! The special [`Undefined`](ValueBox::UNDEFINED) value also exists but does not
//! have a rust equivalent.  It can be created via the [`UNDEFINED`](ValueBox::UNDEFINED)
//! constant.
//!
//! MiniJinja will however create values via an indirection via [`serde`] when
//! a template is rendered or an expression is evaluated.  This can also be
//! triggered manually by using the [`ValueBox::from_serializable`] method:
//!
//! ```
//! # use minijinja::value::ValueBox;
//! let value = ValueBox::from_serializable(&[1, 2, 3]);
//! ```
//!
//! To to into the inverse directly the various [`TryFrom`](std::convert::TryFrom)
//! implementations can be used:
//!
//! ```
//! # use minijinja::value::ValueBox;
//! use std::convert::TryFrom;
//! let v = u64::try_from(ValueBox::from(42)).unwrap();
//! ```
//!
//! # ValueBox Function Arguments
//!
//! [Filters](crate::filters) and [tests](crate::tests) can take values as arguments
//! but optionally also rust types directly.  This conversion for function arguments
//! is performed by the [`FunctionArgs`] and related traits ([`ArgType`], [`FunctionResult`]).
//!
//! # Memory Management
//!
//! ValueBoxs are immutable objects which are internally reference counted which
//! means they can be copied relatively cheaply.  Special care must be taken
//! so that cycles are not created to avoid causing memory leaks.
//!
//! # HTML Escaping
//!
//! MiniJinja inherits the general desire to be clever about escaping.  For this
//! prupose a value will (when auto escaping is enabled) always be escaped.  To
//! prevent this behavior the [`safe`](crate::filters::safe) filter can be used
//! in the template.  Outside of templates the [`ValueBox::from_safe_string`] method
//! can be used to achieve the same result.
//!
//! # Dynamic Objects
//!
//! ValueBoxs can also hold "dynamic" objects.  These are objects which implement the
//! [`Object`] trait and optionally [`SeqObject`] or [`MapObject`]  These can
//! be used to implement dynamic functionality such as stateful values and more.
//! Dynamic objects are internally also used to implement the special `loop`
//! variable or macros.
//!
//! To create a dynamic `ValueBox` object, use [`ValueBox::from_object`],
//! [`ValueBox::from_seq_object`], [`ValueBox::from_map_object`] or the `From<Arc<T:
//! Object>>` implementations for `ValueBox`:
//!
//! ```rust
//! # use std::sync::Arc;
//! # use minijinja::value::{ValueBox, Object};
//! #[derive(Debug)]
//! struct Foo;
//!
//! # impl std::fmt::Display for Foo {
//! #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
//! # }
//! #
//! impl Object for Foo {
//!     /* implementation */
//! }
//!
//! let value = ValueBox::from_object(Foo);
//! let value = ValueBox::from(Arc::new(Foo));
//! let value = ValueBox::from(Arc::new(Foo) as Arc<dyn Object>);
//! ```

// this module is based on the content module in insta which in turn is based
// on the content module in serde::private::ser.

// pub(crate) use crate::value::keyref::KeyRef;
pub(crate) use crate::value::map::{OwnedValueBoxMap, value_map_with_capacity};

pub use crate::value::argtypes::{from_args, ArgType, FunctionArgs, FunctionResult, Kwargs, Rest};
pub use crate::value::object::{Object, SeqObject, SeqObjectIter, MapObject};
pub use crate::value::value::{ValueRepr, ArcCow, ValueKind};

pub(crate) use crate::value::value::{MapType, StringType, Packed, OwnedValueIterator};

mod map;
#[cfg(test)]
mod tests;
mod argtypes;
#[cfg(feature = "deserialization")]
mod deserialize;
mod keyref;
mod object;
pub(crate) mod ops;
mod serialize;
mod value;

pub type ValueBox = Value<'static>;

/// Represents a dynamically typed value in the template engine.
#[derive(Clone)]
pub struct Value<'a>(pub(crate) value::ValueRepr<'a>);

/// Enables value optimizations.
///
/// If `key_interning` is enabled, this turns on that feature, otherwise
/// it becomes a noop.
#[inline(always)]
pub(crate) fn value_optimization() -> impl Drop {
    #[cfg(feature = "key_interning")]
    {
        crate::value::keyref::key_interning::use_string_cache()
    }
    #[cfg(not(feature = "key_interning"))]
    {
        crate::utils::OnDrop::new(|| {})
    }
}

/// Intern a string.
///
/// When the `key_interning` feature is in used, then MiniJinja will attempt to
/// reuse strings in certain cases.  This function can be used to utilize the
/// same functionality.  There is no guarantee that a string will be interned
/// as there are heuristics involved for it.  Additionally the string interning
/// will only work during the template engine execution (eg: within filters etc.).
pub fn intern(s: &str) -> std::sync::Arc<str> {
    #[cfg(feature = "key_interning")]
    {
        crate::value::keyref::key_interning::try_intern(s)
    }
    #[cfg(not(feature = "key_interning"))]
    {
        std::sync::Arc::from(s.to_string())
    }
}

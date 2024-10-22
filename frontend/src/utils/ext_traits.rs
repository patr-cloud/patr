use crate::prelude::*;

/// A trait to extend the [`String`] type with some useful methods that are not
/// available in the standard library. This is useful for adding utility methods
/// to the [`String`] type without polluting the global namespace.
pub trait StringExt: Sized {
	/// Wraps the [`String`] into an option depending on whether it's empty
	/// Returns [`None`] if string is empty otherwise returns the string wrapped
	/// in a [`Some()`]
	fn some_if_not_empty(self) -> Option<Self>;
}

impl StringExt for String {
	fn some_if_not_empty(self) -> Option<Self> {
		if self.is_empty() {
			None
		} else {
			Some(self)
		}
	}
}

impl StringExt for &str {
	fn some_if_not_empty(self) -> Option<Self> {
		if self.is_empty() {
			None
		} else {
			Some(self)
		}
	}
}

/// An extention trait that maps a Signal from one type to another
pub trait SignalMapExt<T>
where
	T: Clone,
{
	/// Maps a signal from one type to another
	fn map<U, F>(self, f: F) -> Signal<U>
	where
		U: Clone,
		F: Fn(T) -> U + 'static;
}

impl<T> SignalMapExt<T> for Signal<T>
where
	T: Clone,
{
	fn map<U, F>(self, f: F) -> Signal<U>
	where
		U: Clone,
		F: Fn(T) -> U + 'static,
	{
		Signal::derive(move || f(self.get()))
	}
}

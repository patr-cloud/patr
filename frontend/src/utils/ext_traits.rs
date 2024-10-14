use crate::prelude::*;

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

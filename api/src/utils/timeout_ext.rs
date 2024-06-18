use std::{future::Future, time::Duration};

use tokio::time::Timeout;

/// An extension trait for [`Future`]s that allows them to be timed out.
pub trait TimeoutExt
where
	Self: Sized,
{
	/// Time out the future after the given duration.
	fn timeout(self, duration: Duration) -> Timeout<Self>;
}

impl<T, O> TimeoutExt for T
where
	T: Future<Output = O>,
{
	fn timeout(self, duration: Duration) -> Timeout<Self> {
		tokio::time::timeout(duration, self)
	}
}

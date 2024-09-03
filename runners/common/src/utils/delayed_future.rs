use std::future::{Future, IntoFuture};

use tokio::time::Instant;

/// A future that resolves to a value at a specific instant in time.
#[derive(Debug, Clone)]
pub struct DelayedFuture<T> {
	/// The instant at which the future will resolve.
	resolve_at: Instant,
	/// The value that the future will resolve to.
	value: T,
}

impl<T> DelayedFuture<T> {
	/// Create a new [`DelayedFuture`] that will resolve to the given value at
	/// the given instant.
	pub fn new(resolve_at: Instant, value: T) -> Self {
		Self { resolve_at, value }
	}

	/// Get the instant at which the future will resolve.
	pub fn resolve_at(&self) -> Instant {
		self.resolve_at
	}

	/// Get the value that the future will resolve to.
	pub fn value(&self) -> &T {
		&self.value
	}
}

impl<T> IntoFuture for DelayedFuture<T> {
	type Output = T;

	type IntoFuture = impl Future<Output = T>;

	fn into_future(self) -> Self::IntoFuture {
		async move {
			tokio::time::sleep_until(self.resolve_at).await;
			self.value
		}
	}
}

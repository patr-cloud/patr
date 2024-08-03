use std::future::{Future, IntoFuture};

use tokio::time::Instant;

#[derive(Debug, Clone)]
pub struct DelayedFuture<T> {
	pub resolve_at: Instant,
	pub value: T,
}

impl<T> DelayedFuture<T> {
	pub fn new(resolve_at: Instant, value: T) -> Self {
		Self { resolve_at, value }
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

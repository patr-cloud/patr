use tokio_util::sync::CancellationToken;

use crate::utils::EitherExt;

/// Extension trait for [`Future`] that provides additional methods. This trait
/// is used to add methods to the [`Future`] type for working with the future
/// and an exit signal, for the sake of convenience.
pub trait ExitableFuture<T> {
	/// Returns the value of the future if it completes before the exit signal
	/// is triggered. If the exit signal is triggered before the future
	/// completes, then this function will return [`None`].
	fn with_cancel_check(self) -> impl Future<Output = Option<T>>;

	/// Returns the value of the future if it completes before the exit signal
	/// is triggered. If the exit signal is triggered before the future
	/// completes, then this function will return [`None`].
	/// This function takes a [`CancellationToken`] as an argument, which
	/// allows you to specify a custom cancellation token for the future.
	/// This is basically the same as
	/// [`with_cancel_check`][1] but with a
	/// different cancellation token than the global one.
	///
	/// [1]: ExitableFuture::with_cancel_check
	fn with_cancel_check_of(
		self,
		cancellation_token: &CancellationToken,
	) -> impl Future<Output = Option<T>>;
}

impl<T, F> ExitableFuture<T> for F
where
	F: Future<Output = T>,
{
	async fn with_cancel_check(self) -> Option<T> {
		self.with_cancel_check_of(crate::GLOBAL_CANCEL_TOKEN.get_or_init(CancellationToken::new))
			.await
	}

	async fn with_cancel_check_of(self, cancellation_token: &CancellationToken) -> Option<T> {
		futures::future::select(
			std::pin::pin!(cancellation_token.cancelled()),
			std::pin::pin!(self),
		)
		.await
		.into_right()
	}
}

use std::future::Future;

use futures::{Stream, StreamExt, future::Either};
use tokio_util::sync::CancellationToken;

use crate::error::RunnerError;

/// Extension trait for [`Either`] that provides additional methods. This trait
/// is used to add methods to the [`Either`] type for working with the left and
/// right variants, for the sake of convenience.
pub trait EitherExt<L, R> {
	/// Extracts the left value from the [`Either`] if it is [`Either::Left`],
	/// otherwise returns [`None`].
	#[allow(dead_code)]
	fn into_left(self) -> Option<L>;
	/// Returns `true` if the [`Either`] is [`Either::Left`], `false` otherwise.
	fn is_left(&self) -> bool;
	/// Extracts the right value from the [`Either`] if it is [`Either::Right`],
	/// otherwise returns [`None`].
	fn into_right(self) -> Option<R>;
	/// Returns `true` if the [`Either`] is [`Either::Right`], `false`
	/// otherwise.
	fn is_right(&self) -> bool;
}

impl<L, NL, R, NR> EitherExt<L, R> for Either<(L, NL), (R, NR)> {
	fn into_left(self) -> Option<L> {
		match self {
			Either::Left((l, _)) => Some(l),
			Either::Right(_) => None,
		}
	}

	fn is_left(&self) -> bool {
		matches!(self, Either::Left(_))
	}

	fn into_right(self) -> Option<R> {
		match self {
			Either::Left(_) => None,
			Either::Right((r, _)) => Some(r),
		}
	}

	fn is_right(&self) -> bool {
		matches!(self, Either::Right(_))
	}
}

/// Extension trait for [`Future`] that provides additional methods. This trait
/// is used to add methods to the [`Future`] type for working with the future
/// and an exit signal, for the sake of convenience.
pub trait ExitableFuture<T> {
	/// Returns the value of the future if it completes before the exit signal
	/// is triggered. If the exit signal is triggered before the future
	/// completes, then this function will return [`None`].
	fn with_cancel_check(self) -> impl Future<Output = Result<T, RunnerError>>;
}

impl<T, F> ExitableFuture<T> for F
where
	F: Future<Output = T>,
{
	async fn with_cancel_check(self) -> Result<T, RunnerError> {
		futures::future::select(
			std::pin::pin!(self),
			std::pin::pin!(
				crate::runner::GLOBAL_CANCEL_TOKEN
					.get_or_init(CancellationToken::new)
					.cancelled()
			),
		)
		.await
		.into_left()
		.ok_or(RunnerError::ExitSignalReceived)
	}
}

/// Extension trait for [`Stream`] that provides additional methods. This trait
/// is used to add methods to the [`Stream`] type for working with the stream
/// and an exit signal, for the sake of convenience.
pub trait ExitableStream<T> {
	/// Iterates through the stream and if the cancel signal is triggered
	/// between items, then the stream will be terminated early. The cancel
	/// signal is not checked while an item is being processed.
	fn with_cancel_check(self) -> impl Stream<Item = T>;
}

impl<T, S> ExitableStream<T> for S
where
	S: Stream<Item = T>,
{
	fn with_cancel_check(self) -> impl Stream<Item = T> {
		self.take_until(
			crate::runner::GLOBAL_CANCEL_TOKEN
				.get_or_init(CancellationToken::new)
				.cancelled(),
		)
	}
}

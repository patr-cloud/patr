use futures::future::Either;

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

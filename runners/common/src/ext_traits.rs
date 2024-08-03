use futures::future::Either;

pub trait EitherExt<L, R> {
	fn into_left(self) -> Option<L>;
	fn is_left(&self) -> bool;
	fn into_right(self) -> Option<R>;
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

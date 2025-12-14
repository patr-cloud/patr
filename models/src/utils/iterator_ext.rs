/// Extension trait for iterators with fallible predicates.
pub trait TryIteratorExt
where
	Self: Iterator,
{
	/// Applies a fallible predicate to all items in the iterator,
	/// returning `Ok(true)` if the predicate returns `true` for all items,
	/// `Ok(false)` if it returns `false` for any item, or an error if the
	/// predicate fails for any item.
	fn try_all<F, E>(&mut self, mut f: F) -> Result<bool, E>
	where
		Self: Sized,
		F: FnMut(Self::Item) -> Result<bool, E>,
	{
		for result in self {
			if !f(result)? {
				return Ok(false);
			}
		}
		Ok(true)
	}

	/// Applies a fallible predicate to all items in the iterator,
	/// returning `Ok(true)` if the predicate returns `true` for any item,
	/// `Ok(false)` if it returns `false` for all items, or an error if the
	/// predicate fails for any item.
	fn try_any<F, E>(&mut self, mut f: F) -> Result<bool, E>
	where
		Self: Sized,
		F: FnMut(Self::Item) -> Result<bool, E>,
	{
		for result in self {
			if f(result)? {
				return Ok(true);
			}
		}
		Ok(false)
	}

	/// Applies a fallible predicate to the items in the iterator, returning the
	/// first item for which the predicate returns `true`. If no such item is
	/// found, returns `Ok(None)`. If the predicate fails for any item, returns
	/// the error.
	fn try_find<F, E>(&mut self, mut f: F) -> Result<Option<Self::Item>, E>
	where
		Self: Sized,
		F: FnMut(&Self::Item) -> Result<bool, E>,
	{
		for item in self {
			if f(&item)? {
				return Ok(Some(item));
			}
		}
		Ok(None)
	}
}

impl<I> TryIteratorExt for I where I: Iterator {}

/// A helper trait for adding a type to a tuple.
/// This is used to add a type to a tuple of types.
///
/// ## Example Usage:
/// ```rust
/// use models::utils::AddTuple;
///
/// type Tuple = (u8, u16, u32);
/// type ResultantTuple = <Tuple as AddTuple<u64>>::ResultantTuple; // (u8, u16, u32, u64)
///
/// assert_eq!(
///     ResultantTuple::default(),
///     (
///         u8::default(),
///         u16::default(),
///         u32::default(),
///         u64::default(),
///     )
/// );
/// ```
pub trait AddTuple<T> {
	/// The resulting tuple after adding the type.
	type ResultantTuple;
}

impl<T> AddTuple<T> for () {
	type ResultantTuple = (T,);
}

/// A macro to implement [`AddTuple`] for tuples of different sizes. It is
/// automatically implemented for tuples of size 1 to 16. More can be added
/// below if you really need it. But if you need more than 16, you should
/// probably rethink your design.
macro_rules! impl_add_tuples {
    ($($header:ident),+ $(,)?) => {
        impl<H, $($header,)*> AddTuple<H> for ($($header,)*) {
            type ResultantTuple = ($($header,)* H);
        }
    };
}

impl_add_tuples!(H1,);
impl_add_tuples!(H1, H2,);
impl_add_tuples!(H1, H2, H3,);
impl_add_tuples!(H1, H2, H3, H4,);
impl_add_tuples!(H1, H2, H3, H4, H5,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8, H9,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15,);
impl_add_tuples!(H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15, H16,);

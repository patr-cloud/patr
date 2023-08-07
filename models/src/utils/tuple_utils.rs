pub trait AddTuple<T> {
	type ResultantTuple;
}

impl AddTuple<()> for () {
	type ResultantTuple = ();
}

// Write a macro to automate the above

macro_rules! impl_add_tuples {
    () => {
        impl<H> AddTuple<H> for () {
            type ResultantTuple = (H,);
        }
    };
    ($($header:ident),+ $(,)?) => {
        impl<H, $($header,)*> AddTuple<H> for ($($header,)*) {
            type ResultantTuple = (H, $($header,)*);
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
impl_add_tuples!(
	H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15,
);
impl_add_tuples!(
	H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12, H13, H14, H15, H16,
);

pub trait LastElementIs<R> {
	fn into_last_element(self) -> R;
	fn get_last_element_ref(&self) -> &R;
	fn get_last_element_mut(&mut self) -> &mut R;
}

macro_rules! impl_last_element_is {
	( $($ty:ident),* $(,)? ) => {
		#[allow(non_snake_case, unused_variables)]
		impl<$($ty,)* R> LastElementIs<R> for ($($ty,)* R,) {
			fn into_last_element(self) -> R {
				let ($($ty,)* item,) = self;
				item
			}
			fn get_last_element_ref(&self) -> &R {
				let ($($ty,)* item,) = &self;
				item
			}
			fn get_last_element_mut(&mut self) -> &mut R {
				let ($($ty,)* item,) = &mut *self;
				item
			}
		}
	}
}

impl_last_element_is!();
impl_last_element_is!(T1);
impl_last_element_is!(T1, T2);
impl_last_element_is!(T1, T2, T3);
impl_last_element_is!(T1, T2, T3, T4);
impl_last_element_is!(T1, T2, T3, T4, T5);
impl_last_element_is!(T1, T2, T3, T4, T5, T6);
impl_last_element_is!(T1, T2, T3, T4, T5, T6, T7);
impl_last_element_is!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_last_element_is!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_last_element_is!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_last_element_is!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_last_element_is!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_last_element_is!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_last_element_is!(
	T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14
);
impl_last_element_is!(
	T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
impl_last_element_is!(
	T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

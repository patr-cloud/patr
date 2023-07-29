/// Extension traits for string. Used for rendering components when the string
/// is not empty.
pub trait StringExt {
	/// Returns `Some(self)` if the string is not empty, `None` otherwise.
	fn some_if_not_empty(self) -> Option<String>;
}

impl StringExt for String {
	fn some_if_not_empty(self) -> Option<String> {
		if self.is_empty() {
			None
		} else {
			Some(self)
		}
	}
}

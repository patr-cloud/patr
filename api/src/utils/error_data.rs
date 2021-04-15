use eve_rs::Error;

pub type EveError = Error<ErrorData>;

#[allow(dead_code)]
#[derive(Default)]
pub struct ErrorData {
	commit_transaction: bool,
}

#[allow(dead_code)]
impl ErrorData {
	fn set_commit_transaction(&mut self, commit: bool) -> &mut Self {
		self.commit_transaction = commit;
		self
	}

	fn should_commit_transaction(&self) -> bool {
		self.commit_transaction
	}
}

pub trait AsErrorData {
	fn commit_transaction(self, commit: bool) -> Self;
}

impl<Value> AsErrorData for Result<Value, Error<ErrorData>> {
	fn commit_transaction(self, commit: bool) -> Self {
		match self {
			Ok(value) => Ok(value),
			Err(mut err) => {
				err.set_commit_transaction(commit);
				Err(err)
			}
		}
	}
}

use eve_rs::Error;

pub type EveError = Error<ErrorData>;

#[allow(dead_code)]
#[derive(Default)]
pub struct ErrorData {
	commit_transaction: bool,
}

#[allow(dead_code)]
impl ErrorData {
	pub fn commit_transaction(&mut self, commit: bool) -> &mut Self {
		self.commit_transaction = commit;
		self
	}

	pub fn should_commit_transaction(&self) -> bool {
		self.commit_transaction
	}
}

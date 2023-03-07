use octorust::types::ReposCreateCommitStatusRequestState;

pub enum CommitStatus {
	// build is started and running
	Running,
	// build finished and success
	Success,
	// build finished and failure
	Failed,
	// build has been errored ie cancelled / internal error
	Errored,
}

impl CommitStatus {
	pub fn commit_state(&self) -> ReposCreateCommitStatusRequestState {
		match self {
			Self::Running => ReposCreateCommitStatusRequestState::Pending,
			Self::Success => ReposCreateCommitStatusRequestState::Success,
			Self::Failed => ReposCreateCommitStatusRequestState::Failure,
			Self::Errored => ReposCreateCommitStatusRequestState::Error,
		}
	}

	pub fn description(&self) -> &str {
		match self {
			Self::Running => "Build is running",
			Self::Success => "Build succeeded",
			Self::Failed => "Build failed",
			Self::Errored => "Error occurred",
		}
	}
}

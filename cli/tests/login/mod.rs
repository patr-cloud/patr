//! `patr login`.

use cli::prelude::*;

use crate::{apply::requests, setup};

/// A runner's token can't log in, and is refused before anything is sent.
#[tokio::test]
async fn login_refuses_a_runner_token() {
	use clap::Parser;

	let server = setup::reset().await;

	let AppArgs { args, command } =
		AppArgs::try_parse_from(["patr", "--token", "patr_rn_runner-token", "login"])
			.expect("failed to parse args");
	let result = cli::commands::execute(command, args, AppState::default()).await;

	assert!(
		matches!(
			&result,
			Err(AppError::ParseError(message)) if message.contains("runner setup reconnect")
		),
		"a `patr_rn_` token wasn't refused: {result:?}"
	);
	assert!(requests(server).await.is_empty());
}

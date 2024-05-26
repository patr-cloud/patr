use std::{marker::PhantomData, str::FromStr};

use futures::StreamExt;
use models::{api::workspace::runner::*, prelude::*, utils::WebSocketUpgrade};

mod client;

#[tokio::main]
async fn main() {
	client::stream_request(
		ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
			.path(StreamRunnerDataForWorkspacePath {
				workspace_id: Uuid::parse_str("00000000000000000000000000000000").unwrap(),
				runner_id: Uuid::parse_str("00000000000000000000000000000000").unwrap(),
			})
			.headers(StreamRunnerDataForWorkspaceRequestHeaders {
				authorization: BearerToken::from_str("Bearer ")
					.expect("Failed to parse Bearer token"),
				user_agent: UserAgent::from_static("runner/docker"),
			})
			.query(())
			.body(WebSocketUpgrade(PhantomData))
			.build(),
	)
	.await
	.expect("Failed to stream request")
	.for_each_concurrent(4, |response| async move {
		println!("{:#?}", response);
	})
	.await;
}

use comfy_table::Table;
use inquire::Select;
use models::api::{user::*, workspace::container_registry::*};

use crate::prelude::*;

/// List all container registry repositories in the workspace
pub(super) async fn execute(
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AuthState::LoggedIn {
		token,
		current_workspace,
	} = state.auth
	else {
		return Err(AppError::NotLoggedIn);
	};

	let workspace_id = if let Some(workspace_id) = current_workspace {
		workspace_id
	} else {
		let workspaces = make_request(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.build(),
		)
		.await?
		.body
		.workspaces;

		let workspace_name = global_args.workspace.unwrap_or_else(|| {
			Select::new(
				"Please select a workspace to use",
				workspaces
					.iter()
					.map(|workspace| workspace.name.clone())
					.collect(),
			)
			.prompt()
			.expect_tty("Failed to read workspace ID")
		});

		workspaces
			.into_iter()
			.find(|workspace| {
				workspace.id.to_string() == workspace_name || workspace.name == workspace_name
			})
			.unwrap_or_else(|| panic!("No workspace found with ID or name: `{workspace_name}`"))
			.id
	};

	let mut repositories = vec![];
	let mut start = 0;

	loop {
		let response = make_request(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath { workspace_id })
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.query(ListResourceQuery {
					page: start / ListResourceQuery::DEFAULT_PAGE_SIZE,
					count: ListResourceQuery::DEFAULT_PAGE_SIZE,
					search: Default::default(),
					sort: Default::default(),
					additional_query: (),
				})
				.build(),
		)
		.await?;

		start += response.body.repositories.len();
		repositories.extend(response.body.repositories);

		if start >= response.headers.total_count.0 {
			break;
		}
	}

	let formatted_rows = repositories
		.iter()
		.map(|repo| {
			[
				repo.id.to_string(),
				repo.name.clone(),
				format_size(repo.size),
				repo.last_updated.to_string(),
			]
		})
		.collect::<Vec<_>>();

	CommandOutput::builder()
		.text(
			Table::new()
				.set_header(["ID", "Name", "Size", "Last Updated"])
				.add_rows(formatted_rows)
				.to_string(),
		)
		.json(ListContainerRepositoriesResponse { repositories }.to_json_value())
		.build()
		.into_result()
}

/// Format a byte count as a human-readable string (GiB / MiB / KiB / B).
fn format_size(bytes: u64) -> String {
	const KIB: u64 = 1024;
	const MIB: u64 = KIB * 1024;
	const GIB: u64 = MIB * 1024;

	if bytes >= GIB {
		format!("{:.2} GiB", bytes as f64 / GIB as f64)
	} else if bytes >= MIB {
		format!("{:.2} MiB", bytes as f64 / MIB as f64)
	} else if bytes >= KIB {
		format!("{:.2} KiB", bytes as f64 / KIB as f64)
	} else {
		format!("{} B", bytes)
	}
}

use crate::prelude::*;

#[server(UpdateDatabaseFn, endpoint = "/infrastructure/database/update")]
pub async fn update_database(
	_access_token: Option<String>,
	_database_id: Option<Uuid>,
	_workspace_id: Option<Uuid>,
) -> Result<(), ServerFnError<ErrorType>> {
	Ok(()).map_err(ServerFnError::WrappedServerError)
}

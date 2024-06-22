use leptos_use::utils::FromToStringCodec;

use crate::prelude::*;

pub fn get_workspaces() {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);

	let workspace_list = create_resource(
		move || access_token.get(),
		move |value| async move { list_user_workspace(value).await },
	);

	let x = workspace_list.get().unwrap().unwrap();
}

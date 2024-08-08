use models::{api::user::UserApiToken, prelude::*};
use time::format_description;

use crate::prelude::*;

#[component]
pub fn ApiTokenCard(
	/// Additional class names to apply to the row, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The User API Token
	#[prop(into)]
	token: MaybeSignal<WithId<UserApiToken>>,
) -> impl IntoView {
	let outer_class = class.with(|cname| {
		format!(
			"bg-secondary-light w-full row-card px-xl border border-border-color \
			cursor-pointer br-bottom-sm flex items-center justify-center {}",
			cname
		)
	});

	let navigate = use_navigate();

	let format =
		format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
	let date = match token.get().data.created.format(&format) {
		Ok(date) => date.into_view(),
		Err(_) => "Invalid Date".into_view(),
	};

	let expiry = match token.get().data.token_exp {
		Some(expiry) => match expiry.format(&format) {
			Ok(date) => date.into_view(),
			Err(_) => "Invalid Date".into_view(),
		},
		None => "Never".into_view(),
	};

	let token_id = token.get().id.to_string();

	view! {
		<tr
			on:click={move |_| navigate(format!("/user/api-tokens/{}", token_id).as_str(), Default::default())}
			tab_index=0
			class={outer_class}
			aria_label="Select API Token"
		>
			<td class="flex-4 flex items-center justify-center">{token.get().data.name}</td>
			<td class="flex-4 flex items-center justify-center">{expiry.clone()}</td>
			<td class="flex-4 flex items-center justify-center">{date.clone()}</td>
		</tr>
	}
}

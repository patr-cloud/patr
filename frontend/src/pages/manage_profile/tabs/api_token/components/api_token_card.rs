use ev::MouseEvent;
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

	let store_token = store_value(token.clone());

	let navigate = use_navigate();

	let format =
		format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();

	let date = move || match store_token.with_value(|token| {
		let format =
			format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();

		token.get().data.created.clone().format(&format.clone())
	}) {
		Ok(date) => date.into_view(),
		Err(_) => "Invalid Date".into_view(),
	};

	let expiry = move || match store_token.with_value(|token| token.get().data.token_exp) {
		Some(expiry) => match expiry.format(&format.clone()) {
			Ok(date) => date.into_view(),
			Err(_) => "Invalid Date".into_view(),
		},
		None => "Never".into_view(),
	};

	let token_id =
		Signal::derive(move || store_token.with_value(|token| token.get().id.to_string()));

	let on_click_link = move |ev: MouseEvent| {
		ev.prevent_default();
		logging::log!("/user/api-tokens/{}", token_id.get());
		navigate(
			format!("/user/api-tokens/{}", token_id.get()).as_str(),
			Default::default(),
		);
	};

	view! {
		<tr on:click={on_click_link} tab_index=0 class={outer_class} aria_label="Select API Token">
			<td class="flex-4 flex items-center justify-center">
				{move || store_token.with_value(|token| token.get().name.clone())}
			</td>
			<td class="flex-4 flex items-center justify-center">{expiry.clone()}</td>
			<td class="flex-4 flex items-center justify-center">{date.clone()}</td>
		</tr>
	}
}

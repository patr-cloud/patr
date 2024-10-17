use leptos_query::*;

use crate::{
	prelude::*,
	queries::{list_api_tokens_query, AllApiTokensTag},
};

mod components;
mod create_token;
mod edit_token;
mod utils;

use self::components::*;
pub use self::{create_token::*, edit_token::*};

/// Container for the API tokens tab
#[component]
pub fn ApiTokensTab() -> impl IntoView {
	view! {
		<div class="flex flex-col items-start justify-start w-full h-full px-md py-xl px-md gap-md">
			<Outlet />
		</div>
	}
}

/// List all the API tokens
#[component]
pub fn ListApiTokens() -> impl IntoView {
	let QueryResult {
		data: token_list, ..
	} = list_api_tokens_query().use_query(move || AllApiTokensTag);

	view! {
		<Link r#type={Variant::Link} style_variant={LinkStyleVariant::Contained} to="create">
			"Create New Token"
			<Icon
				icon={IconType::Plus}
				size={Size::ExtraSmall}
				class="ml-xs"
				color={Color::Black}
			/>
		</Link>

		<Transition>
			{move || match token_list.get() {
				Some(token_list) => {
					match token_list {
						Ok(data) => {
							view! {
								<TableDashboard
									column_grids={vec![4, 4, 4]}
									headings={vec![
										"Name".into_view(),
										"Expiry".into_view(),
										"Created At".into_view(),
									]}
									render_rows={
										view! {
											<For
												each={move || data.clone().tokens}
												key={|state| state.id}
												let:child
											>
												<ApiTokenCard token={child} />
											</For>
										}
										.into_view()
									}
								/>
							}
						}
						Err(_) => view! {}.into_view(),
					}
				}
				None => view! {}.into_view(),
			}}
		</Transition>
	}
}

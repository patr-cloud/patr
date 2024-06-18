use codee::string::FromToStringCodec;
use leptos_use::use_cookie;
use models::api::user::ListApiTokensResponse;

use crate::prelude::*;

mod api_token_card;
mod edit_token;

pub use self::{api_token_card::*, edit_token::*};

#[component]
pub fn ApiTokensTab() -> impl IntoView {
	view! {
		<div class="fc-fs-fs full-width full-height px-md py-xl gap-md">
			<Outlet />
		</div>
	}
}

#[component]
pub fn ListApiTokens() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let access_token_signal = move || access_token.get();
	let token_list = create_resource(access_token_signal, move |value| async move {
		load_api_tokens_list(value).await
	});

	// let l = token_list.get();
	// logging::log!("{:#?}", token_list.get());

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
			{
				move || match token_list.get() {
					Some(token_list) => {
						match token_list {
							Ok(data) => {
								view! {
									<TableDashboard
										column_grids={vec![4, 4, 4]}
										headings={vec!["Name".into_view(), "Expiry".into_view(), "Created At".into_view()]}

										render_rows={view! {
											<For
												each={move || data.clone().tokens}
												key={|state| state.id}
												let:child
											>
												<ApiTokenCard token={child}/>
											</For>
										}
											.into_view()}
									/>
								}
							}
							Err(_) => view! {}.into_view(),
						}
					},
					None => view! {}.into_view(),
				}
			}
		</Transition>
	}
}

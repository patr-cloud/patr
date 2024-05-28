use leptos_use::{use_cookie, utils::FromToStringCodec};
use models::api::user::GetUserInfoResponse;

use crate::prelude::*;

#[component]
pub fn ProfileSettings() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>("access_token");
	let access_token_signal = move || access_token.get();
	let user_data = create_resource(access_token_signal, move |value| async move {
		load_user_data(value).await
	});

	view! {
		<div class="full-width fit-wide-screen mx-auto fc-fs-fs px-md my-xl gap-lg">
			<Transition>

				{move || match user_data.get() {
					Some(user_data) => {
						logging::log!("{:#?}", user_data);
						match user_data {
							Ok(data) => {
								view! {
									<BasicInfo basic_user_info={data.clone().basic_user_info}/>
									<ContactInfo user_email={data.clone().recovery_email}/>
								}
									.into_view()
							}
							Err(_) => view! {}.into_view(),
						}
					}
					None => view! {}.into_view(),
				}}

			</Transition>
			<PasswordSection/>
		</div>
	}
}

mod basic_info;
mod contact_info;
mod email_card;
mod password_section;

pub use self::{basic_info::*, contact_info::*, email_card::*, password_section::*};

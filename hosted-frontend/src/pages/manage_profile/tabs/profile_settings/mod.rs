use crate::prelude::*;

/// The Profile Settings Page, shows the basic info about the user, contact info
/// and password management
#[component]
pub fn ProfileSettings() -> impl IntoView {
	let access_token_signal = move || AuthState::load().0.get().get_access_token();
	let user_data = create_resource(access_token_signal, move |value| async move {
		load_user_data(value).await
	});

	view! {
		<div class="w-full fit-wide-screen mx-auto flex flex-col items-start justify-start px-md my-xl gap-lg">
			<Transition>
				{move || match user_data.get() {
					Some(user_data) => {
						logging::log!("{:#?}", user_data);
						match user_data {
							Ok(data) => {
								view! {
									<BasicInfo basic_user_info={data.clone().basic_user_info} />
									<ContactInfo user_email={data.clone().recovery_email} />
								}
									.into_view()
							}
							Err(_) => view! {}.into_view(),
						}
					}
					None => view! {}.into_view(),
				}}

			</Transition>
			<PasswordSection />
		</div>
	}
}

mod basic_info;
mod contact_info;
mod email_card;
mod password_section;

pub use self::{basic_info::*, contact_info::*, email_card::*, password_section::*};

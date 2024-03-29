use leptos_use::{use_cookie, utils::FromToStringCodec};

use crate::prelude::*;

#[component]
pub fn ProfileSettings() -> impl IntoView {
	view! {
		<div
			class="full-width fit-wide-screen mx-auto fc-fs-fs px-md my-xl gap-lg"
		>
			<BasicInfo />
			<ContactInfo />
			<PasswordSection />
		</div>
	}
}

mod basic_info;
mod contact_info;
mod email_card;
mod password_section;

pub use self::{basic_info::*, contact_info::*, email_card::*, password_section::*};

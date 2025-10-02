use log::info;

use crate::prelude::*;

/// Home Page
#[component]
pub fn HomePage() -> impl IntoView {
	info!("hello");
	view! {
		<div class="pt-[25vh] w-full flex justify-center items-center">
			<h1 class="text-primary text-xl">"Welcome To Patr!"</h1>

			<div>
				<Input
					r#type={InputType::Text}
				/>
				<PasswordInput />
			</div>
		</div>
	}
}

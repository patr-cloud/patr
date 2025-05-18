use crate::prelude::*;
use log::info;

/// Home Page
#[component]
pub fn HomePage() -> Element {
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

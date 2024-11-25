use rand::Rng;

use crate::prelude::*;

#[component]
pub fn HomePage() -> impl IntoView {
	let toaster = expect_toaster();
	let mut rng = rand::thread_rng();

	view! {
		<div class="pt-[25vh] w-full flex justify-center items-center">
			<h1 class="text-primary text-xl">"Welcome To Patr!"</h1>

			<div>
				<button
					on:click=move |ev| {
						ev.prevent_default();
						let random = rng.gen::<u32>();
						toaster.toast(
							ToastData::builder()
								.message(format!("Hello World {}", random).as_str())
								.expiry(None)
								.level(AlertType::Success)
								.dismissible(true),
						);
					}
				>
					"click"
				</button>
			</div>
		</div>
	}
}

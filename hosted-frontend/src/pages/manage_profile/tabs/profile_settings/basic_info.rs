use models::api::user::BasicUserInfo;

use crate::prelude::*;

/// Basic Info about the user, contains name and email id
#[component]
pub fn BasicInfo(
	/// Basic User with Id
	// #[prop(into)]
	basic_user_info: WithId<BasicUserInfo>,
) -> impl IntoView {
	view! {
		<section class="text-white flex flex-col items-start justify-start w-full px-xl py-lg br-sm bg-secondary-light">
			<div class="flex items-start justify-center w-full border-b border-border-color pb-sm">
				<h2 class="tracking-[1px] text-md">"Basic Info"</h2>
			</div>

			<form class="w-full flex flex-col items-start justify-start gap-md pt-md">
				<div class="flex w-full px-md">
					<div class="flex-col-2 fr-fs-fs">
						<label html_for="username" class="mt-sm txt-sm">
							"Username"
						</label>
					</div>

					<div class="flex-col-10 flex flex-col items-start justify-start">
						<Input
							id="username"
							disabled=true
							class="w-full"
							placeholder="Enter Username"
							variant={SecondaryColorVariant::Medium}
							value={basic_user_info.clone().data.username}
						/>
					</div>
				</div>

				<div class="flex w-full px-md">
					<div class="flex-col-2 flex items-start justify-start">
						<label html_for="username" class="mt-sm text-sm">
							"Name"
						</label>
					</div>

					<div class="flex-col-5 flex flex-col items-start justify-start pr-xs">
						<Input
							id="first_name"
							disabled=true
							class="w-full"
							placeholder="First Name"
							variant={SecondaryColorVariant::Medium}
							value={basic_user_info.clone().data.first_name}
						/>
					</div>

					<div class="flex-col-5 flex flex-col items-start justify-start pl-xs">
						<Input
							id="last_name"
							disabled=true
							class="w-full"
							placeholder="Last Name"
							variant={SecondaryColorVariant::Medium}
							value={basic_user_info.clone().data.last_name}
						/>
					</div>
				</div>
			</form>
		</section>
	}
}

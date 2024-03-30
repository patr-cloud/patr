use models::api::user::BasicUserInfo;

use crate::prelude::*;

#[component]
pub fn BasicInfo(
	/// Basic User with Id
	// #[prop(into)]
	basic_user_info: WithId<BasicUserInfo>,
) -> impl IntoView {
	view! {
		<section class="txt-white fc-fs-fs full-width px-xl py-lg br-sm bg-secondary-light">
			<div class="fr-fs-ct full-width pb-sm ul-light">
				<h2 class="letter-sp-md txt-md">"Basic Info"</h2>
			</div>

			<form class="full-width gap-md fc-fs-fs pt-md">
				<div class="flex full-width px-md">
					<div class="flex-col-2 fr-fs-fs">
						<label html_for="username" class="mt-sm txt-sm">
							"Username"
						</label>
					</div>

					<div class="flex-col-10 fc-fs-fs">
						<Input
							id="username"
							disabled=true
							class="full-width"
							placeholder="Enter Username"
							variant=SecondaryColorVariant::Medium
							value={basic_user_info.clone().data.username}
						/>
					</div>
				</div>

				<div class="flex full-width px-md">
					<div class="flex-col-2 fr-fs-fs">
						<label html_for="username" class="mt-sm txt-sm">
							"Name"
						</label>
					</div>

					<div class="flex-col-5 fc-fs-fs pr-xs">
						<Input
							id="first_name"
							disabled=true
							class="full-width"
							placeholder="First Name"
							variant=SecondaryColorVariant::Medium
							value={basic_user_info.clone().data.first_name}
						/>
					</div>

					<div class="flex-col-5 fc-fs-fs pl-xs">
						<Input
							id="last_name"
							disabled=true
							class="full-width"
							placeholder="Last Name"
							variant=SecondaryColorVariant::Medium
							value={basic_user_info.clone().data.last_name}
						/>
					</div>
				</div>
			</form>
		</section>
	}
}

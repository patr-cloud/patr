use crate::prelude::*;

mod choose_permission;
mod create_token;
mod permission_card;
mod permission_item;

pub use self::{choose_permission::*, create_token::*, permission_card::*, permission_item::*};

#[component]
pub fn EditApiToken() -> impl IntoView {
	view! {
		<div class="full-width fit-wide-screen full-height txt-white fc-fs-fs px-md">
			<div class="fr-fs-ct mb-md full-width">
				<p class="txt-md">
					<strong class="txt-md">"Manage Token"</strong>
				</p>

				<Link style_variant={LinkStyleVariant::Contained} class="ml-auto">
					"REGENERATE TOKEN"
				</Link>

				<button class="btn btn-error ml-md">"REVOKE TOKEN"</button>
			</div>

			<div class="flex mb-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-fs pt-xs">
					<label html_for="name" class="txt-white txt-sm">
						"Token Name"
					</label>
				</div>

				<div class="flex-col-10 fc-fs-fs pl-xl">
					<Input
						r#type={InputType::Text}
						placeholder="Enter Token Name"
						class="full-width"
					/>
				</div>
			</div>

			<div class="flex mb-xs full-width mb-md">
				<div class="flex-col-2 fc-fs-fs pt-xs">
					<label html_for="allowedIps" class="txt-white txt-sm">
						"Allowed IP(s)"
					</label>
					<small class="txt-xxs txt-grey">
						"By default, all IP addresses will be allowed."
					</small>
				</div>
				<div class="flex-col-10 fc-fs-fs pl-xl">
					<Input
						r#type={InputType::Text}
						placeholder="Enter Allowed IP addresses"
						class="full-width"
					/>
				</div>
			</div>

			<div class="flex mb-xs full-width mb-md">
				<div class="flex-col-2 fc-fs-fs pt-xs">
					<label html_for="tokenNbf" class="txt-white txt-sm">
						"Token Validity"
					</label>
					<small class="txt-xxs txt-grey">
						"By default, the token will be valid forever from the date created."
					</small>
				</div>
				<div class="flex-col-10 fr-fs-ct pl-xl">
					<div class="flex-col-1 fr-ct-ct txt-sm">"Valid from"</div>
					<div class="flex-col-5 fr-fs-fs pl-md">
						<Input
							r#type={InputType::Date}
							placeholder="Valid From"
							class="full-width cursor-text"
						/>
					</div>
					<div class="flex-col-1 fr-ct-ct txt-sm">"to"</div>
					<div class="flex-col-5 fr-fs-fs pl-md">
						<Input
							r#type={InputType::Date}
							placeholder="Valid Till"
							class="full-width cursor-text"
						/>
					</div>
				</div>
			</div>

			<div class="fc-fs-fs mb-xs full-width my-md gap-sm">
				<label class="txt-white txt-sm">"Choose Permissions"</label>
				<div class="full-width fc-fs-fs gap-xs">
					<PermisisonCard/>
				</div>
			</div>

			<div class="full-width fr-fe-ct py-md mt-auto">
				<Link class="txt-sm txt-medium mr-sm">"BACK"</Link>
				<Link style_variant={LinkStyleVariant::Contained} class="txt-sm txt-medium mr-sm">
					"UPDATE"
				</Link>
			</div>
		</div>
	}
}

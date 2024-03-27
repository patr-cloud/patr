use crate::prelude::*;

#[component]
pub fn ConfirmSignUpPage() -> impl IntoView {
	view! {
		<PageContainer class="bg-image">
			<ConfirmSignUpForm />
		</PageContainer>
	}
}

#[component]
pub fn ConfirmSignUpForm() -> impl IntoView {
	view! {
		<div class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">
					"Confirm OTP"
				</h1>

				<div class="txt-primary txt-thin fr-fs-fs">
					<Link
						to="/login"
						r#type=Variant::Link
						class="ml-xs"
					>
						"Sign Up with different Email"
					</Link>
				</div>
			</div>

			<form class="fc-fs-fs full-width">
			</form>
		</div>
	}
}

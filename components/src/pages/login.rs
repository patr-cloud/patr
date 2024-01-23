use crate::imports::*;

#[component]
fn LoginForm() -> impl IntoView {
	view! {
		<form class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">"Sign In"</h1>
				<div class="txt-white txt-thin fr-fs-fs">
					<p>"New User? "</p>
					<Link to="/sign-up" variant=Variant::Link>
						"Sign Up"
					</Link>
				</div>
			</div>

			<div class="fc-fs-fs full-width gap-md">
				<Input
					class="full-width"
					id="username"
					r#type=InputType::Email
					placeholder="Username/Email"
					start_icon=Some(
						IconProps::builder().icon(IconType::User).size(Size::ExtraSmall).build(),
					)
				/>

				<Input
					class="full-width"
					id="password"
					r#type=InputType::Password
					placeholder="Password"
					start_icon=Some(
						IconProps::builder().icon(IconType::Shield).size(Size::ExtraSmall).build(),
					)
				/>

			</div>

			<div class="fr-sb-ct full-width pt-xs">
				<Link
					to="https://book.leptos.dev/view/03_components.html".to_owned()
					variant=Variant::Link
				>
					"Forgot Password?"
				</Link>
			</div>
			<Link
				r#type="submit"
				variant=Variant::Button
				class="btn ml-auto mt-md"
				style_variant=LinkStyleVariant::Contained
			>
				"LOGIN"
			</Link>
		</form>
	}
}

#[component]
pub fn LoginPage() -> impl IntoView {
	view! {
		<PageContainer class="bg-onboard">
			<LoginForm/>
		</PageContainer>
	}
}

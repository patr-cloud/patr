use std::rc::Rc;

use crate::imports::*;

#[component]
fn LoginForm() -> impl IntoView {
	let is_js_enable = create_rw_signal(false);
	let show_password = create_rw_signal(false);

	// FIX: use create_effect rather render_effect.
	create_render_effect(move |_| {
		is_js_enable.set(true);
		// log(format!("{:?}", is_js_enable))
	});

	view! {
		<form class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">"Sign In"</h1>
				<div class="txt-white txt-thin fr-fs-fs">
					<p>"New User? "</p>
					<Link
						to={"/sign-up"}
						variant={ Variant::Link }
					>
						"Sign Up"
					</Link>
				</div>
			</div>

			<div class="fc-fs-fs full-width gap-md">
				<Input
					class="full-width"
					id="username"
					r#type="email"
					placeholder="Username/Email"
					start_icon={
						Some(
							IconProps::builder()
								.icon(IconType::User)
								.size(Size::ExtraSmall)
								.build()
						)
					}
				/>
				<Input
					class="full-width"
					id="password"
					r#type="password"
					placeholder="Password"
					start_icon={
						Some(
							IconProps::builder()
								.icon(IconType::Shield)
								.size(Size::ExtraSmall)
								.build()
						)
					}
					// FIX: is_js_enable is not reactive
					end_icon={
						if is_js_enable.get() {
							Some(
								IconProps::builder()
									.icon(MaybeSignal::derive(move || {
										if show_password.get() {
											IconType::Eye
										} else {
											IconType::EyeOff
										}
									}))
									.size(Size::ExtraSmall)
									.on_click(Rc::new(move |_| {
										show_password.update(|val| *val = !*val);
									}))
									.build()
							)
						} else {
							None
						}
					}
				/>
			</div>

			<div class="fr-sb-ct full-width pt-xs">
				<Link
					to={"https://book.leptos.dev/view/03_components.html".to_owned()}
					variant={Variant::Link}
				>
					"Forgot Password?"
				</Link>

				// <label for="remember-me" class="fr-fs-ct txt-primary cursor-pointer">
				//     <Input
				//         r#type="checkbox"
				//         id="remember-me"
				//         class="mr-xs"
				//         value="Remember Me"
				//     />
				//     "Remember Me"
				// </label>
			</div>
			<Link
				r#type="submit"
				variant={Variant::Button}
				class="btn ml-auto mt-md"
				style_variant={LinkStyleVariant::Contained}
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
			<LoginForm />
		</PageContainer>
	}
}

use web_sys::SubmitEvent;

use crate::prelude::*;

/// The Login Page
#[component]
pub fn LoginPage() -> impl IntoView {
	view! {
		<PageContainer class="bg-onboard">
			<LoginForm />
		</PageContainer>
	}
}

/// The login form component. This is the form that the user uses to log in to
/// the application.
#[component]
pub fn LoginForm() -> impl IntoView {
	let on_submit_login = move |ev: SubmitEvent| {};

	view! {
		<form on:submit={on_submit_login} class="box-onboard text-white">
			<div class="flex justify-between items-baseline mb-lg w-full">
				<h1 class="text-primary text-xl text-medium">"Sign In"</h1>
				<div class="text-white text-thin flex items-start justify-start">
					<p>"New User? "</p>
					// <Link to={"/sign-up".to_owned()} r#type={Variant::Link}>
					// 	"Sign Up"
					// </Link>
				</div>
			</div>

			<div class="flex flex-col items-start justify-start w-full gap-md">
				<Input
					id="user_id"
					name="user_id"
					class="w-full"
					r#type={InputType::Text}
					placeholder="Username / Email"
					disabled={false}
					start_icon={|| view! {
						<Icon
							icon={IconType::User}
							size={Size::ExtraSmall}
						/>
					}}
				/>

				// {move || username_error
				// 	.get()
				// 	.some_if_not_empty()
				// 	.map(|message| view! {
				// 		<Alert r#type={AlertType::Error} class="mt-xs">
				// 			{&message}
				// 		</Alert>
				// 	})}

				<PasswordInput
					class="w-full"
					id="password"
					placeholder="Password"
					start_icon={|| view! {
						<Icon
							icon={IconType::Shield}
							size={Size::ExtraSmall}
						/>
					}}
				/>

				<input name="mfa_otp" type="hidden" />

				// {move || password_error
				// 	.get()
				// 	.some_if_not_empty()
				// 	.map(|message| view! {
				// 		<Alert r#type={AlertType::Error} class="mt-xs">
				// 			{&message}
				// 		</Alert>
				// 	})}
			</div>


			// {move || if loading.get() { view! {
			// 		<Spinner class="ml-auto" />
			// 	}
			// } else {
			// 	view! {
			// 		<Link
			// 			should_submit=true
			// 			r#type={Variant::Button}
			// 			class="btn ml-auto mt-md"
			// 			style_variant={LinkStyleVariant::Contained}
			// 			>
			// 			"LOGIN"
			// 		</Link>
			// 	}
			// }}
		</form>
	}
}

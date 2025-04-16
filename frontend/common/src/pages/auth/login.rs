use leptos::ev::{Event, SubmitEvent};

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
	let username = RwSignal::new("".to_owned());
	let password = RwSignal::new("".to_owned());

	let username_error = RwSignal::new("".to_owned());
	let password_error = RwSignal::new("".to_owned());

	let loading = RwSignal::new(false);

	let on_submit_login = move |ev: SubmitEvent| {
		ev.prevent_default();

		loading.set(true);
		username_error.set("".to_owned());
		password_error.set("".to_owned());

		if username.get().is_empty() {
			log::error!("no email");
			username_error.set("Username cannot be empty".to_owned());
			loading.set(false);
		}

		if password.get().is_empty() {
			log::error!("no password");
			password_error.set("Password cannot be empty".to_owned());
			loading.set(false);
		}

		// TODO: Submit Form Here
		log::info!("Submit Form");
	};

	let username_error_alert = move || {
		username_error.get().some_if_not_empty().map(|val| {
			view! {
				<Alert r#type={AlertType::Error} class="mt-xs">
					{val}
				</Alert>
			}
		})
	};

	let password_error_alert = move || {
		password_error.get().some_if_not_empty().map(|val| {
			view! {
				<Alert r#type={AlertType::Error} class="mt-xs">
					{val}
				</Alert>
			}
		})
	};

	view! {
		<form on:submit={on_submit_login} class="box-onboard text-white">
			<div class="flex justify-between items-baseline mb-lg w-full">
				<h1 class="text-primary text-xl text-medium">"Sign In"</h1>
				<div class="text-white text-thin flex items-start justify-start text-sm">
					<p>"New User? "</p>
					<Link to={"/sign-up".to_owned()}>
						"Sign Up"
					</Link>
				</div>
			</div>

			<div class="flex flex-col items-start justify-start w-full gap-md">
				<Input
					value={username}
					on_input={move |ev: Event| {
						username.set(event_target_value(&ev));
					}}
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

				{username_error_alert}

				<PasswordInput
					on_input={move |ev| {
						password.set(event_target_value(&ev));
					}}
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

				{password_error_alert}
			</div>


			<Show
				when={move || !loading.get()}
				fallback={|| view! {
					<p>"Loading..."</p>
				}}
			>
				<Button
					r#type={ButtonType::Submit}
					class="btn ml-auto mt-md"
					variant={LinkStyleVariant::Contained}
				>
					"LOGIN"
				</Button>
			</Show>
		</form>
	}
}

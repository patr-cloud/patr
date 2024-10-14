use std::rc::Rc;

use super::DatabaseParams;
use crate::prelude::*;

#[component]
pub fn ChangePasswordButton() -> impl IntoView {
	let show_password_modal = create_rw_signal(false);
	let change_password_action = create_server_action::<UpdateDatabaseFn>();

	let (state, _) = AuthState::load();
	let access_token = Signal::derive(move || state.get().get_access_token());
	let current_workspace_id = Signal::derive(move || state.get().get_last_used_workspace_id());

	let params = use_params::<DatabaseParams>();
	let database_id = Signal::derive(move || {
		params.with(|params| {
			params
				.as_ref()
				.map(|param| param.database_id.clone().unwrap_or_default())
				.unwrap_or_default()
		})
	});

	view! {
		<Show when={move || show_password_modal.get()}>
			<Modal color_variant={SecondaryColorVariant::Light}>
				<div
					style="width: 30%;"
					class="center-modal txt-white txt-sm fc-fs-fs bg-secondary-light br-sm p-xl show-center-modal gap-lg"
				>
					<h3 class="txt-primary txt-lg">"Change Password"</h3>

					<ActionForm action={change_password_action} class="fc-fs-fs gap-lg full-width">
						<input type="hidden" name="access_token" prop:value={access_token} />
						<input type="hidden" name="database_id" prop:value={database_id} />
						<input
							type="hidden"
							name="workspace_id"
							prop:value={current_workspace_id
								.map(|value| value.map(|value| value.to_string()))}
						/>

						<div class="fc-fs-fs gap-md full-width">
							<label class="txt-white">"Change the password for this database"</label>
							<Input
								variant={SecondaryColorVariant::Medium}
								class="full-width"
								r#type={InputType::Password}
								placeholder="Enter New Password"
								name="password"
								id="password"
							/>
						</div>

						<div class="fc-fs-fs full-width">
							<Input
								variant={SecondaryColorVariant::Medium}
								class="full-width"
								r#type={InputType::Password}
								placeholder="Confirm New Password"
								name="confirm_pass"
								id="confirm_pass"
							/>
						</div>

						<div class="full-width fr-fe-ct mt-auto gap-md">
							<Link
								should_submit=false
								class="txt-white"
								on_click={Rc::new(move |_| show_password_modal.set(false))}
							>
								"CANCEL"
							</Link>
							<Link should_submit=true style_variant={LinkStyleVariant::Contained}>
								"CONFIRM"
							</Link>
						</div>
					</ActionForm>
				</div>
			</Modal>
		</Show>
		<div class="fr-sb-fs full-width gap-md mt-xxs">
			<Link
				on_click={Rc::new(move |_| { show_password_modal.set(true) })}
				class="txt-medium ml-auto"
			>
				"CHANGE PASSWORD"
			</Link>
		</div>
	}
}

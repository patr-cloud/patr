use crate::prelude::*;

#[component]
pub fn TopNav(
	/// The scope of the component
	cx: Scope,
) -> impl IntoView {
	let open_feedback = create_rw_signal(cx, false);
	let show_profile_settings = create_rw_signal(cx, false);

	view! { cx,
		<header class="full-width fr-sb-ct pt-xl pb-md">
			<nav class="full-width fr-fe-ct">
				<button
					on:click=move |_| open_feedback.set(true)
					class="btn btn-secondary row-card mx-sm"
				>
					Feedback
				</button>
				{move || open_feedback.get().then(move || {
					view! { cx,
						// <FeedbackModal
						// 	handleSubmit={handleSubmit}
						// 	openFeedback={openFeedback}
						// 	setOpenFeedback={setOpenFeedback}
						// 	state={state}
						// />
					}
				})}
				<ProfileCard
					on_click=Box::new(move |_| {
						show_profile_settings.set(true)
					}) />
			</nav>
		</header>
	}
}

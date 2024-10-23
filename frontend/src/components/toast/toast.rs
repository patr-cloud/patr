use ev::MouseEvent;
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};

use crate::imports::*;

fn is_container_empty() -> bool {
	expect_toaster().queue.get().is_empty()
}

#[component]
pub fn Toaster() -> impl IntoView {
	let toaster = expect_toaster();

	view! {
		<Portal>
			<Show
				when=move || !is_container_empty()
			>
				 <div class="toaster-container">
					<div class="toaster">
						<For
							each=move || toaster.queue.get()
							key=|toast| toast.id
							let:toast
						>
							<Toast
								toast_data=toast
							/>
						</For>
					</div>
				</div>
			</Show>
		</Portal>
	}
}

/// The Toast component. The toast component is used to display a notification
/// to the user. For Example, to show the user a notification for success
/// or failure of an action like reseouce creation or deletion.
#[component]
pub fn Toast(
	/// The Toast Data
	toast_data: ToastData,
) -> impl IntoView {
	// {} {} popup br-sm fixed flex flex-col items-start justify-start text-white
	// outline-primary-focus",
	let handle_click = move |_: MouseEvent| {
		if !toast_data.dismissible {
			return;
		}

		toast_data.clear.set(true);
	};

	let expiry = f64::from(toast_data.expiry.unwrap_or(0));

	let UseTimeoutFnReturn { start, .. } = use_timeout_fn(
		move |_: ()| {
			toast_data.clear.set(true);
		},
		expiry,
	);

	create_effect(move |_| {
		if toast_data.expiry.is_none() {
			return;
		}
		start(());
	});

	let class = format!("toast bg-{}", toast_data.level.as_css_name(),);

	let UseTimeoutFnReturn { start, .. } = use_timeout_fn(
		move |_: ()| {
			expect_toaster().remove(toast_data.id);
		},
		175 as f64,
	);

	create_effect(move |_| {
		if toast_data.clear.get() {
			start(());
		}
	});

	view! {
		<div
			on:click=handle_click
			class=class
		>
			{toast_data.message}
		</div>
	}
}

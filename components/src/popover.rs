use leptos::{window, *};
use web_sys::DomRect;

use crate::imports::*;

#[derive(Default, PartialEq, Clone)]
pub enum PopoverTriggerType {
	#[default]
	Hover,
	Click,
}

/// A Component to Show Popovers, Takes two children,
/// the first one will be the `trigger_children` for example a button, the
/// second children will be the content for the popover.
#[component]
pub fn Popover(
	/// The Child for the popover, e.g. A button
	#[prop(into)]
	trigger_children: View,
	/// Classes to be applied to the trigger
	#[prop(into, optional)]
	trigger_class: MaybeSignal<String>,
	/// The Content of the popover, e.g. a description of what this button does
	#[prop(into)]
	popover_content: View,
	/// The Placement of the Popover
	#[prop(into, optional)]
	popover_placement: PopoverPlacement,
) -> impl IntoView {
	let show_popover = create_rw_signal(false);
	let on_mouse_enter = move |_| {
		show_popover.set(true);
	};
	let on_mouse_leave = move |_| {
		show_popover.set(false);
	};

	let trigger_ref = NodeRef::<html::Div>::new();
	let popover_ref = NodeRef::<html::Div>::new();

	let get_popover_position = move || {
		let Some(popover_rect) = popover_ref.get() else {
			return (0., 0.);
		};
		let popover_rect = popover_rect.get_bounding_client_rect();
		let Some(trigger_rect) = trigger_ref.get() else {
			return (0., 0.);
		};
		let trigger_rect = trigger_rect.get_bounding_client_rect();
		let (top, left, transform) = match popover_placement {
			PopoverPlacement::Top => {
				let Some(window_height) = window_height() else {
					return (0., 0.);
				};

				let popover_height = popover_rect.height();
				let trigger_top = trigger_rect.top();
				let trigger_bottom = trigger_rect.bottom();
				let top = trigger_top - popover_height;

				let top = if top < 0. && trigger_bottom + popover_height <= window_height {
					trigger_bottom
				} else {
					top
				};

				let left = trigger_rect.left() + trigger_rect.width() / 2.;
				let transform = "translateX(-50%)".to_string();

				(top, left, transform)
			}
			PopoverPlacement::Bottom => {
				let Some(window_height) = window_height() else {
					return (0., 0.);
				};

				let popover_height = popover_rect.height();
				let trigger_top = trigger_rect.top();
				let trigger_bottom = trigger_rect.bottom();
				let top = trigger_bottom;

				let top =
					if top + popover_height > window_height && trigger_top - popover_height >= 0. {
						trigger_top - popover_height
					} else {
						top
					};

				let left = trigger_rect.left() + trigger_rect.width() / 2.;
				let transform = "translateX(-50%)".to_string();

				(top, left, transform)
			}
			PopoverPlacement::Left => {
				let Some(window_width) = window_width() else {
					return (0., 0.);
				};

				let popover_width = popover_rect.width();
				let trigger_left = trigger_rect.left();
				let trigger_right = trigger_rect.right();
				let left = trigger_left - popover_width;

				let left = if left < 0. && trigger_right + popover_width <= window_width {
					trigger_right
				} else {
					left
				};

				let top = trigger_rect.top() + trigger_rect.height() / 2.;
				let transform = "translateY(-50%)".to_string();

				(top, left, transform)
			}
			PopoverPlacement::Right => {
				let Some(window_width) = window_width() else {
					return (0., 0.);
				};

				let popover_width = popover_rect.width();
				let trigger_left = trigger_rect.left();
				let trigger_right = trigger_rect.right();

				let left = trigger_right;

				let left =
					if left + popover_width > window_width && trigger_left - popover_width >= 0. {
						trigger_left - popover_width
					} else {
						left
					};

				let top = trigger_rect.top() + trigger_rect.height() / 2.;
				let transform = "translateY(-50%)".to_string();

				(top, left, transform)
			}
		};

		(top, left)
	};

	view! {
		<div
			class={trigger_class}
			ref={trigger_ref}
			on:mouseenter={on_mouse_enter}
			on:mouseleave={on_mouse_leave}
		>
			{trigger_children}
			<Portal>
				<div
					ref={popover_ref}
					style={
						let (top, left) = get_popover_position();
						logging::log!("{}, {}", top, left);
						format!("top:{top}px;left:{left}px;")
					}
				>
					{popover_content.clone()}
				</div>
			</Portal>
		</div>
	}
	.into_view()
}

/// The Placement of the Popover relative to the trigger
#[derive(Clone, PartialEq, Default)]
pub enum PopoverPlacement {
	#[default]
	Top,
	Bottom,
	Left,
	Right,
}

pub struct PopoverPlacementOffset {
	pub top: f64,
	pub left: f64,
	pub transform: String,
}

pub fn get_popover_offset(
	trigger_rect: &DomRect,
	popover_rect: &DomRect,
	placement: PopoverPlacement,
) -> Option<(f64, f64)> {
	let (top, left, transform) = match placement {
		PopoverPlacement::Top => {
			let Some(window_height) = window_height() else {
				return None;
			};

			let popover_height = popover_rect.height();
			let trigger_top = trigger_rect.top();
			let trigger_bottom = trigger_rect.bottom();
			let top = trigger_top - popover_height;

			let top = if top < 0. && trigger_bottom + popover_height <= window_height {
				trigger_bottom
			} else {
				top
			};

			let left = trigger_rect.left() + trigger_rect.width() / 2.;
			let transform = "translateX(-50%)".to_string();

			(top, left, transform)
		}
		PopoverPlacement::Bottom => {
			let Some(window_height) = window_height() else {
				return None;
			};

			let popover_height = popover_rect.height();
			let trigger_top = trigger_rect.top();
			let trigger_bottom = trigger_rect.bottom();
			let top = trigger_bottom;

			let top = if top + popover_height > window_height && trigger_top - popover_height >= 0.
			{
				trigger_top - popover_height
			} else {
				top
			};

			let left = trigger_rect.left() + trigger_rect.width() / 2.;
			let transform = "translateX(-50%)".to_string();

			(top, left, transform)
		}
		PopoverPlacement::Left => {
			let Some(window_width) = window_width() else {
				return None;
			};

			let popover_width = popover_rect.width();
			let trigger_left = trigger_rect.left();
			let trigger_right = trigger_rect.right();
			let left = trigger_left - popover_width;

			let left = if left < 0. && trigger_right + popover_width <= window_width {
				trigger_right
			} else {
				left
			};

			let top = trigger_rect.top() + trigger_rect.height() / 2.;
			let transform = "translateY(-50%)".to_string();

			(top, left, transform)
		}
		PopoverPlacement::Right => {
			let Some(window_width) = window_width() else {
				return None;
			};

			let popover_width = popover_rect.width();
			let trigger_left = trigger_rect.left();
			let trigger_right = trigger_rect.right();

			let left = trigger_right;

			let left = if left + popover_width > window_width && trigger_left - popover_width >= 0.
			{
				trigger_left - popover_width
			} else {
				left
			};

			let top = trigger_rect.top() + trigger_rect.height() / 2.;
			let transform = "translateY(-50%)".to_string();

			(top, left, transform)
		}
	};

	Some((top, left))
}

fn window_height() -> Option<f64> {
	window()
		.inner_height()
		.map(|height| height.as_f64())
		.ok()
		.flatten()
}

fn window_width() -> Option<f64> {
	window()
		.inner_width()
		.map(|width| width.as_f64())
		.ok()
		.flatten()
}

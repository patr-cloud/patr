use leptos_use::use_event_listener;

use crate::prelude::*;

/// Tooltip for displaying additional information when hovering over an element
#[component]
pub fn Tooltip(
	/// The content of the tooltip
	#[prop(into, optional)]
	content: String,
	/// The color variant of the tooltip
	#[prop(into, optional)]
	variant: SecondaryColorVariant,
	/// The parent ref to use for positioning the tooltip
	#[prop(into)]
	parent_ref: NodeRef<html::Span>,
	/// The children of the tooltip, if any
	#[prop(into)]
	children: ChildrenFn,
) -> impl IntoView {
	let tooltip_ref = create_node_ref::<html::Span>();

	let is_visible = create_rw_signal(false);

	create_effect(move |_| {
		let show_tooltip = move || {
			is_visible.set(true);
		};
		let hide_tooltip = move || {
			is_visible.set(false);
		};

		let handle_mouse_move = move |e: ev::MouseEvent| {
			if let Some((tooltip_ref, parent_ref)) = tooltip_ref.get().zip(parent_ref.get()) {
				if is_visible.get() {
					let root_font_size_in_px = window()
						.get_computed_style(&document().body().unwrap())
						.unwrap()
						.and_then(|style| style.get_property_value("font-size").ok());
					if let Some(root_font_size_in_px) = root_font_size_in_px {
						let root_font_size = root_font_size_in_px.parse::<f64>().unwrap();
						let tooltip_dimensions = tooltip_ref.get_bounding_client_rect();
						let container_dimensions = parent_ref.get_bounding_client_rect();
						let client_x = e.client_x() as f64;
						let client_y = e.client_y() as f64;

						let is_outside_tooltip =
							{ client_y + 2.6 * root_font_size < tooltip_dimensions.top() } ||
								{
									client_y - 0.6 * root_font_size > tooltip_dimensions.bottom()
								} || { client_x + 0.6 * root_font_size < tooltip_dimensions.left() } ||
								{ client_x - 0.6 * root_font_size > tooltip_dimensions.right() };
						let is_outside_container =
							{ client_y + 0.6 * root_font_size < container_dimensions.top() } ||
								{
									client_y - 0.6 * root_font_size > container_dimensions.bottom()
								} || { client_x + 0.6 * root_font_size < container_dimensions.left() } ||
								{
									client_x - 0.6 * root_font_size > container_dimensions.right()
								};

						if is_outside_tooltip && is_outside_container {
							hide_tooltip();
						}
					}
				}
			}
		};

		_ = use_event_listener(window(), ev::mousemove, handle_mouse_move);
		_ = use_event_listener(parent_ref, ev::mouseenter, move |_| show_tooltip());
		_ = use_event_listener(parent_ref, ev::focus, move |_| show_tooltip());
		_ = use_event_listener(parent_ref, ev::blur, move |_| hide_tooltip());
	});

	let get_tooltip_dimensions = move || {
		if let Some(r#ref) = tooltip_ref.get() {
			let root_font_size_in_px = window()
				.get_computed_style(&document().body().unwrap())
				.unwrap()
				.and_then(|style| style.get_property_value("font-size").ok());
			if let Some(root_font_size_in_px) = root_font_size_in_px {
				let root_font_size = root_font_size_in_px.parse::<f64>().unwrap();
				let bounding_rect = r#ref.get_bounding_client_rect();

				let parent_left = bounding_rect.left();
				let parent_top = bounding_rect.top();
				let parent_width = r#ref.offset_width() as f64;
				let parent_height = r#ref.offset_height() as f64;

				let tooltip_width = 16f64 * root_font_size;
				(
					parent_top + parent_height + root_font_size,
					parent_left + parent_width / 2f64 - tooltip_width / 2f64,
					tooltip_width,
				)
			} else {
				(0f64, 0f64, 0f64)
			}
		} else {
			(0f64, 0f64, 0f64)
		}
	};

	view! {
		<Portal>
			<span
				style=move || {
					let (top, left, width) = get_tooltip_dimensions();
					format!("top: {}; left: {}; width: {}", top, left, width)
				}
				ref={tooltip_ref}
				class=move || format!(
					"tooltip pos-fix br-sm {} {}",
					if is_visible.get() {
						"tooltip-visible"
					} else {
						""
					},
					variant.as_css_name(),
				)
			>
				<span class="tip px-md py-xxs full-width fc-fs-fs pos-rel">
					<span class="fr-ct-ct txt-white full-width txt-xxs">
						{&content}
						{children()}
					</span>
				</span>
			</span>
		</Portal>
	}
}

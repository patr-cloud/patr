use leptos_use::{use_document, use_event_listener, use_window};

use crate::imports::*;

#[component]
pub fn Tooltip(
	/// The Content of the tooltip
	#[prop(into, optional)]
	content: String,
	/// The Children of the tooltip, if any
	#[prop(into)]
	children: ChildrenFn,
	/// Parent Ref used for positioning the tooltip, typically the
	/// <ToolTipContainer />
	parent_ref: NodeRef<html::Span>,
	/// The Color Variant of the tooltip
	variant: SecondaryColorVariant,
	/// The Width of the tooltip
	width: f64,
) -> impl IntoView {
	let tooltip_ref = create_node_ref::<html::Span>();
	let is_visible = create_rw_signal(false);

	let show_tooltip = move || is_visible.set(true);

	let hide_tooltip = move || {
		is_visible.set(false);
	};

	let on_mouse_move = move |ev: ev::MouseEvent| {
		let window = use_window();
		if window.is_none() && window.document().is_none() {
			return;
		}
		if let Some((tooltip_ref, parent_ref)) =
			tooltip_ref.get_untracked().zip(parent_ref.get_untracked())
		{
			if is_visible.get_untracked() {
				let document = window.document();

				let root_font_size = window
					.as_ref()
					.unwrap()
					.get_computed_style(&document.body().unwrap())
					.unwrap()
					.and_then(|sty| sty.get_property_value("font-size").ok());

				if let Some(root_font_size) = root_font_size {
					let root_font_size = root_font_size.parse::<f64>().unwrap_or(16.0);
					let tooltip_dim = tooltip_ref.get_bounding_client_rect();
					let parent_dim = parent_ref.get_bounding_client_rect();

					let mouse_x = ev.client_x() as f64;
					let mouse_y = ev.client_y() as f64;

					let mouse_outside_tooltip =
						{ mouse_y + 2.6 * root_font_size < tooltip_dim.top() } ||
							{ mouse_y - 0.6 * root_font_size > tooltip_dim.bottom() } ||
							{ mouse_x + 0.6 * root_font_size < tooltip_dim.left() } ||
							{ mouse_x - 0.6 * root_font_size > tooltip_dim.right() };

					let mouse_outside_container =
						{ mouse_y + 0.6 * root_font_size < parent_dim.top() } ||
							{ mouse_y - 0.6 * root_font_size > parent_dim.bottom() } ||
							{ mouse_x + 0.6 * root_font_size < parent_dim.left() } ||
							{ mouse_x - 0.6 * root_font_size > parent_dim.right() };

					if mouse_outside_container && mouse_outside_tooltip {
						hide_tooltip();
					}
				}
			}
		}
	};

	let get_tooltip_dimensions = move || {
		let window = use_window();
		let document = window.document();

		if window.is_none() || document.is_none() {
			return (0., 0., 0.);
		}

		let tip_ref = tooltip_ref.get();
		let cont_ref = parent_ref.get();
		if tip_ref.is_none() || cont_ref.is_none() {
			return (0., 0., 0.);
		}

		let tip_ref = tip_ref.unwrap();
		let cont_ref = cont_ref.unwrap();

		let root_font_size = window
			.as_ref()
			.unwrap()
			.get_computed_style(&document.body().unwrap())
			.unwrap()
			.and_then(|sty| sty.get_property_value("font-size").ok());

		let root_font_size = root_font_size
			.unwrap_or("16.0".to_owned())
			.as_str()
			.parse::<f64>()
			.unwrap_or(16.0);

		let bounding_rect = cont_ref.get_bounding_client_rect();

		let parent_left = bounding_rect.left();
		let parent_top = bounding_rect.top();
		let parent_width = tip_ref.offset_width() as f64;
		let parent_height = tip_ref.offset_height() as f64;

		let tooltip_width = width * root_font_size;

		(
			parent_top + parent_height + root_font_size,
			parent_left + parent_width / 2. - tooltip_width / 2.,
			tooltip_width,
		)
	};

	_ = use_event_listener(use_document(), ev::mousemove, on_mouse_move);
	_ = use_event_listener(parent_ref, ev::mouseenter, move |_| show_tooltip());
	_ = use_event_listener(parent_ref, ev::focus, move |_| show_tooltip());
	_ = use_event_listener(parent_ref, ev::blur, move |_| hide_tooltip());

	view! {
		<Portal>
			<span
				style={move || {
					let (top, left, width) = get_tooltip_dimensions();
					logging::log!("{}, {}, {}", top, left, width);
					format!("top: {top}px; width: {width}px; left: {left}px")
				}}

				ref={tooltip_ref}
				class={move || {
					format!(
						"tooltip pos-fix br-sm {} {}",
						if is_visible.get() { "tooltip-visible" } else { "" },
						variant.as_css_name(),
					)
				}}
			>

				<span class="tip mx-md py-xxs full-width fc-fs-fs pos-rel">
					<span class="fr-ct-ct txt-white full-width txt-xxs">
						{&content} {children()}
					</span>
				</span>
			</span>
		</Portal>
	}
}

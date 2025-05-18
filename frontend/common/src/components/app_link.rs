use dioxus_router::components::Link;

use crate::prelude::*;

/// Link component to navigate to other pages, wraps around HTML a tag, with
/// additional props for styling and such
#[component]
pub fn AppLink(
	/// The Children of the Link, usually a \<p\> tag or simply
	/// the link text
	children: Element,
	/// Additional class names to apply to the link, if any
	#[props(into, optional)]
	class: String,
	/// Variant of the Link
	#[props(into, optional)]
	variant: LinkStyleVariant,
	/// The Target of the Link, to be used with the link variant
	#[props(into)]
	to: NavigationTarget,
	/// Color of the link
	#[props(into, optional)]
	color: Color,
	/// A class to apply to the generate HTML anchor tag if the `target` route
	/// is active.
	active_class: Option<String>,
	/// When [`true`], the `target` route will be opened in a new tab.
	///
	/// This does not change whether the [`Link`] is active or not.
	#[props(default)]
	new_tab: bool,
	/// The onclick event handler.
	onclick: Option<EventHandler<MouseEvent>>,
	/// The onmounted event handler.
	/// Fired when the <a> element is mounted.
	onmounted: Option<EventHandler<MountedEvent>>,
	#[props(default)]
	/// Whether the default behavior should be executed if an `onclick` handler
	/// is provided.
	///
	/// 1. When `onclick` is [`None`] (default if not specified), `onclick_only`
	///    has no effect.
	/// 2. If `onclick_only` is [`false`] (default if not specified), the
	///    provided `onclick` handler will be executed after the links regular
	///    functionality.
	/// 3. If `onclick_only` is [`true`], only the provided `onclick` handler
	///    will be executed.
	onclick_only: bool,
	/// The rel attribute for the generated HTML anchor tag.
	///
	/// For external `a`s, this defaults to `noopener noreferrer`.
	rel: Option<String>,
) -> Element {
	rsx! {
		Link {
			class: format!(
			    "flex items-center justify-center {class} {}",
			    match variant {
			        LinkStyleVariant::Outlined => "btn-outline".to_string(),
			        LinkStyleVariant::Contained => format!("btn btn-{color}"),
			        LinkStyleVariant::Plain => format!("btn-plain text-{color}"),
			    },
			),
			to,
			active_class,
			new_tab,
			onclick,
			onmounted,
			onclick_only,
			rel,
			{children}
		}
	}
}

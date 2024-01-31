use crate::imports::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Status {
	Deleted,
	Errored,
	Created,
	Pushed,
	#[default]
	Stopped,
	Deploying,
	Running,
	Live,
}

impl Status {
	pub const fn get_status_color(self) -> &'static str {
		match self {
			Self::Deleted => "bg-error",
			Self::Errored => "bg-error",
			Self::Created => "bg-info",
			Self::Pushed => "bg-info",
			Self::Stopped => "bg-grey",
			Self::Deploying => "bg-warning",
			Self::Running => "bg-success",
			Self::Live => "bg-success",
		}
	}

	pub const fn get_status_text(self) -> &'static str {
		match self {
			Self::Deleted => "deleted",
			Self::Errored => "error",
			Self::Created => "created",
			Self::Pushed => "pushed",
			Self::Stopped => "stopped",
			Self::Deploying => "deploying",
			Self::Running => "running",
			Self::Live => "live",
		}
	}
}

#[component]
pub fn StatusBadge(
	/// Additional Classed to add, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Status of the component
	#[prop(into, optional)]
	status: MaybeSignal<Status>,
	/// Background color of the badge
	#[prop(into, optional)]
	color: MaybeSignal<Color>,
	/// Text of the badge
	#[prop(into, optional)]
	text: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"status-badge pos-rel txt-secondary cursor-default {} {}",
			// color.get().as_css_name(),
			status.get().get_status_color(),
			class.get(),
		)
	};

	view! {
		<span class=class>
			// {text.get()}
			{status.get().get_status_text()}
		</span>
	}
}

use crate::imports::*;

/// The Status of the component
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub enum Status {
	/// Indicates that the component has been deleted
	Deleted,
	/// Indicates that the component has faced an error
	Errored,
	/// Indicates that the component has been created
	Created,
	/// Indicates that the component has been pushed
	Pushed,
	/// Indicates that the component has been stopped
	#[default]
	Stopped,
	/// Indicates that the component is deploying
	Deploying,
	/// Indicates that the component is running
	Running,
	/// Indicates that the component is live
	Live,
}

impl Status {
	/// Gets the css class name color of the status badge
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

	/// Get the status text of the status badge
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
) -> impl IntoView {
	let class = move || {
		format!(
			"status-badge pos-rel txt-secondary cursor-default {} {}",
			status.get().get_status_color(),
			class.get(),
		)
	};

	view! {
		<span class=class>
			{status.get().get_status_text()}
		</span>
	}
}

use std::collections::HashSet;

use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

use crate::models::File;

#[allow(dead_code)]
pub struct FileList {
	link: ComponentLink<Self>,
	files: HashSet<File>,
}

#[derive(Properties, Clone)]
pub struct FileListProps {
	pub files: HashSet<File>,
}

impl Component for FileList {
	type Message = ();
	type Properties = FileListProps;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		Self {
			link,
			files: props.files,
		}
	}

	fn update(&mut self, _: Self::Message) -> ShouldRender {
		false
	}

	fn change(&mut self, _: Self::Properties) -> ShouldRender {
		false
	}

	fn view(&self) -> Html {
		html! {
			<>
			{{
				self.files
					.iter()
					.map(File::to_html)
					.collect::<Vec<_>>()
			}}
			</>
		}
	}
}

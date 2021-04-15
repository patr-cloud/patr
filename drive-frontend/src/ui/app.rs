use std::collections::HashSet;

use yew::prelude::*;

use crate::{
	models::File,
	ui::{FileList, Sidebar},
};

#[allow(dead_code)]
pub struct App {
	link: ComponentLink<Self>,
	files: HashSet<File>,
}

impl Component for App {
	type Message = ();
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let mut files = HashSet::new();
		files.insert(File {
			file_id: String::from("0"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("1"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("2"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("3"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("4"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("5"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("6"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("7"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("8"),
			name: String::from("test.txt"),
		});
		files.insert(File {
			file_id: String::from("9"),
			name: String::from("test.txt"),
		});
		App { link, files }
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
				<nav>
				<div class="nav-wrapper container">
					<a href="#" class="brand-logo">{{"Drive"}}</a>
					<div class="row">
						<div class="input-field col s8 push-s2">
							<input type="text" class="autocomplete white-text white-border" id="search-autocomplete-input" />
							<label for="search-autocomplete-input">{{"Search"}}</label>
						</div>
					</div>
				</div>
				</nav>

				<div class="row">
					<div class="col s2" style="padding: 0px; margin: 0px;">
						<div class="row"/>
						<div class="row">
							<a class="col push-s5 btn-floating btn-large waves-effect waves-light blue">{{"+"}}</a>
						</div>
						<Sidebar selected_item="My Drive" />
					</div>
					<div class="container col s10 push-s1">
						<div class="row" />
						<FileList files={{&self.files}}/>
					</div>
				</div>
			</>
		}
	}

	fn rendered(&mut self, _: bool) {
		crate::init_materialize();
	}
}

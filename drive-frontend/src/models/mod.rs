use std::hash::{Hash, Hasher};

use yew::{html, Html};

#[derive(Clone)]
pub struct File {
	pub file_id: String,
	pub name: String,
}

impl File {
	pub fn to_html(&self) -> Html {
		html! {
			<div class="col s2">
				<div class="card small hoverable">
					<div class="card-image">
						<img src="folder-icon.png" style="" />
					</div>
					<div class="card-content">
						{{&self.name}}
					</div>
				</div>
			</div>
		}
	}
}

impl Hash for File {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.file_id.hash(state)
	}
}

impl PartialEq for File {
	fn eq(&self, other: &Self) -> bool {
		self.file_id.eq(&other.file_id)
	}
}

impl Eq for File {}

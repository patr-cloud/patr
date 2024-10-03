use std::sync::OnceLock;

static REQWEST_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub async fn make_request(url: &str) -> Result<reqwest::Response, reqwest::Error> {
	let client = REQWEST_CLIENT.get_or_init(|| reqwest::Client::new());
	client.get(url).send().await
}

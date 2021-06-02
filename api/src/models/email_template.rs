use lettre::message::MultiPart;
pub use api_macros::EmailTemplate;

use crate::utils::Error;

#[async_trait::async_trait]
pub trait EmailTemplate {
	async fn render_body(&self) -> Result<MultiPart, Error>;
}

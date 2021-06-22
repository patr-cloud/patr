pub use api_macros::EmailTemplate;
use lettre::message::MultiPart;

use crate::utils::Error;

#[async_trait::async_trait]
pub trait EmailTemplate {
	async fn render_body(&self) -> Result<MultiPart, Error>;
}

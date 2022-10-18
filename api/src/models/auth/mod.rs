mod access_token_data;
mod user_api_token_data;

pub use self::{access_token_data::*, user_api_token_data::*};

#[derive(Clone, Debug)]
pub enum UserAuthenticationData {
	AccessToken(AccessTokenData),
	ApiToken(UserApiTokenData),
}



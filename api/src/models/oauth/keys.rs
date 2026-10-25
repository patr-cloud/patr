use jsonwebtoken::{
	Algorithm,
	EncodingKey,
	jwk::{Jwk, PublicKeyUse, ThumbprintHash},
};

use crate::{prelude::*, utils::config::AppConfig};

/// A signing keypair, as declared in `oauth.signingKeys`.
pub struct SigningKey {
	/// The JOSE `kid`: the key's RFC 7638 thumbprint.
	pub kid: String,
	/// The key material, ready to sign with.
	pub encoding_key: EncodingKey,
	/// The public half, ready to publish.
	pub jwk: Jwk,
}

/// Every signing key in the config, in declared order.
fn load_keys(config: &AppConfig) -> Result<Vec<SigningKey>, ErrorType> {
	config
		.oauth
		.signing_keys
		.iter()
		.map(|pem| {
			let encoding_key =
				EncodingKey::from_ec_pem(pem.as_bytes()).map_err(ErrorType::server_error)?;
			let mut jwk = Jwk::from_encoding_key(&encoding_key, Algorithm::ES256)
				.map_err(ErrorType::server_error)?;
			let kid = jwk
				.thumbprint(ThumbprintHash::SHA256)
				.map_err(ErrorType::server_error)?;
			jwk.common.key_id = Some(kid.clone());
			jwk.common.public_key_use = Some(PublicKeyUse::Signature);

			Ok(SigningKey {
				kid,
				encoding_key,
				jwk,
			})
		})
		.collect()
}

/// The first key declared signs. To rotate, append the new key and deploy so
/// every relying party picks it up from the JWKS, then move it to the front.
pub fn get_signing_key(config: &AppConfig) -> Result<SigningKey, ErrorType> {
	load_keys(config)?
		.into_iter()
		.next()
		.ok_or_else(|| ErrorType::server_error("no OAuth signing key is configured"))
}

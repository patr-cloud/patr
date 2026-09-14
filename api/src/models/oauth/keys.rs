use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use jsonwebtoken::{
	Algorithm,
	DecodingKey,
	EncodingKey,
	jwk::{AlgorithmParameters, Jwk, PublicKeyUse},
};
use sha2::{Digest, Sha256};

use crate::{prelude::*, utils::config::AppConfig};

/// A signing keypair, as declared in `oauth.signingKeys`.
pub struct SigningKey {
	/// The JOSE `kid`.
	pub kid: String,
	/// The key material, ready to sign with.
	pub encoding_key: EncodingKey,
	/// The public half, ready to publish.
	pub jwk: Jwk,
}

impl SigningKey {
	/// The public half, for verifying.
	pub fn decoding_key(&self) -> Result<DecodingKey, ErrorType> {
		DecodingKey::from_jwk(&self.jwk)
			.inspect_err(|err| {
				error!(
					"Error building a decoding key for kid `{}`: {}",
					self.kid, err
				);
			})
			.map_err(ErrorType::server_error)
	}
}

/// Every signing key in the config, in declared order. Parsed per call:
/// `from_ec_pem` only decodes the base64, so a cache would buy nothing.
fn load_keys(config: &AppConfig) -> Result<Vec<SigningKey>, ErrorType> {
	config
		.oauth
		.signing_keys
		.iter()
		.map(|pem| {
			let encoding_key = EncodingKey::from_ec_pem(pem.as_bytes())
				.inspect_err(|err| {
					error!("Error reading an OAuth signing key: {}", err);
				})
				.map_err(ErrorType::server_error)?;

			let mut jwk = Jwk::from_encoding_key(&encoding_key, Algorithm::ES256)
				.inspect_err(|err| {
					error!("Error deriving the public half of a signing key: {}", err);
				})
				.map_err(ErrorType::server_error)?;

			let kid = thumbprint(&jwk)?;
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

/// The RFC 7638 thumbprint, used as the `kid`. Derived from the key so an
/// operator can't pick a colliding one or forget to change it on rotation.
fn thumbprint(jwk: &Jwk) -> Result<String, ErrorType> {
	let AlgorithmParameters::EllipticCurve(ec) = &jwk.algorithm else {
		error!("An OAuth signing key is not an elliptic curve key");
		return Err(ErrorType::server_error("unsupported signing key type"));
	};

	// RFC 7638 section 3.2: required members only, lexically ordered, no
	// whitespace. Written out so the ordering can't drift with serde.
	let canonical = format!(
		r#"{{"crv":"{}","kty":"EC","x":"{}","y":"{}"}}"#,
		serde_json::to_value(&ec.curve)
			.ok()
			.and_then(|value| value.as_str().map(ToOwned::to_owned))
			.unwrap_or_default(),
		ec.x,
		ec.y,
	);

	Ok(BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(canonical.as_bytes())))
}

/// The first key declared signs. To rotate, append the new key and deploy so
/// every relying party picks it up from the JWKS, then move it to the front.
pub fn get_signing_key(config: &AppConfig) -> Result<SigningKey, ErrorType> {
	load_keys(config)?.into_iter().next().ok_or_else(|| {
		error!("No OAuth signing key is configured — set `oauth.signingKeys`");
		ErrorType::server_error("no OAuth signing key is configured")
	})
}

/// Finds the key a token names in its `kid`, if it is one of ours.
pub fn get_key_by_id(config: &AppConfig, kid: &str) -> Result<Option<SigningKey>, ErrorType> {
	Ok(load_keys(config)?.into_iter().find(|key| key.kid == kid))
}

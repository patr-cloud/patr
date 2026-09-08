//! Clears every web login so no access token minted under the old `sub`
//! meaning survives the deploy.
//!
//! `AccessTokenData.sub` used to hold the login id. It now holds the user id,
//! with the login id moved to a new `sid` claim, because OpenID Connect
//! requires `sub` to identify the *user* — relying parties key their local
//! accounts on it, so a value that changed per session would mint a new
//! account on every sign-in.
//!
//! A token issued under the old meaning is never misread: it has no `sid`, a
//! required claim, so it fails to decode, and `authenticate_jwt` rejects any
//! token whose `sub` isn't the user owning its `sid`. Either way it comes back
//! as `malformedAccessToken`, which the dashboard doesn't refresh on — it
//! clears its auth state and sends the user to log in again. So the deploy
//! logs every user out regardless. Deleting the sessions makes that one clean
//! logout, and takes their refresh tokens with them, which would otherwise
//! stay usable for up to 30 days after it.
//!
//! `user_login` rows are deleted alongside, since `web_login` is a subtype of
//! it and would otherwise leave parents with no child. API tokens are a
//! different subtype and carry no JWT, so they are deliberately untouched.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Ordered child-first: web_login points at user_login, which in turn
	// points at actor_client.
	sqlx::query(
		r#"
		DELETE FROM web_login;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// `audit_log.actor_client_id` references `actor_client`, so the registry
	// rows outlive the logins that created them and only `user_login` is
	// removed here. A tombstoned actor_client with no user_login is inert —
	// nothing can authenticate as it — and keeping it preserves the audit
	// trail of what that session did.
	sqlx::query(
		r#"
		DELETE FROM
			user_login
		WHERE
			login_type = 'web_login';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

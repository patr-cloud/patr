//! Clears every web login so no access token minted under the old `sub`
//! meaning survives the deploy.
//!
//! `AccessTokenData.sub` used to hold the login id. It now holds the user id,
//! with the login id moved to a new `sid` claim, because OpenID Connect
//! requires `sub` to identify the *user* — relying parties key their local
//! accounts on it, so a value that changed per session would mint a new
//! account on every sign-in.
//!
//! Both values are `UUID`, so a token issued under the old meaning does not
//! fail validation after the change — it is silently *misread*, with a login
//! id interpreted as a user id. That is cross-user misidentification rather
//! than a clean rejection, which is why the sessions cannot simply be left to
//! expire on their own. Deleting them makes every stale token unresolvable:
//! the authenticator looks the session up by `sid`, and there is no row.
//!
//! Every user is logged out by this migration. Refresh tokens go with the
//! rows, so clients cannot silently renew either — they get a fresh login.
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

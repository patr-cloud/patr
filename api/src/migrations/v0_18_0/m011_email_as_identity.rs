//! Collapses a user's three identity handles — `username`, `recovery_email`,
//! and `recovery_phone_number` — down to a single `email` column that is both
//! their identifier and their only contact address.
//!
//! The phone half of the old model never worked: there is no SMS sender, so a
//! phone-only account could never actually receive an OTP and could never
//! reset its password. The multi-value `user_email` / `user_phone_number`
//! tables only ever held the one row that `"user".recovery_*` pointed at, and
//! the `user_unverified_*` tables had no writer at all.
//!
//! `recovery_email` is **renamed** rather than replaced, so the data stays put
//! and the column keeps its ordinal position — a freshly initialized schema
//! and a migrated one end up identical, which matters because `SELECT
//! "user".*` decodes positionally against the offline sqlx cache.
//!
//! The column also becomes `CITEXT`, so `WHERE email = $1` and the unique
//! constraint are both case-insensitive without every call site having to
//! remember to lowercase first.
//!
//! Only phone-only accounts need backfilling, and there is deliberately no
//! `DELETE` for rows that still end up without an email — the `SET NOT NULL`
//! is the guard. If it trips, the deploy stops and the offending rows get
//! looked at by hand rather than being silently destroyed.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// These all reference columns that are about to be renamed or dropped, and
	// the FK into `user_email` has to go before that table can.
	sqlx::query(
		r#"
		ALTER TABLE "user"
			DROP CONSTRAINT user_fk_id_recovery_email,
			DROP CONSTRAINT user_fk_id_recovery_phone_country_code_recovery_phone_number,
			DROP CONSTRAINT user_uq_username,
			DROP CONSTRAINT user_uq_recovery_email,
			DROP CONSTRAINT user_uq_recovery_phone_country_code_recovery_phone_number,
			DROP CONSTRAINT user_chk_username_is_valid,
			DROP CONSTRAINT user_chk_recovery_email_is_lower_case,
			DROP CONSTRAINT user_chk_recovery_phone_country_code_is_upper_case,
			DROP CONSTRAINT user_chk_email_or_phone_present;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE "user"
			RENAME COLUMN recovery_email TO email;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Phone-only accounts have no recovery_email; fall back to whatever
	// `user_email` row they had before that table goes away.
	sqlx::query(
		r#"
		UPDATE
			"user"
		SET
			email = (
				SELECT
					user_email.email
				FROM
					user_email
				WHERE
					user_email.user_id = "user".id
				ORDER BY
					user_email.email
				LIMIT 1
			)
		WHERE
			email IS NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE "user"
			DROP COLUMN username,
			DROP COLUMN recovery_phone_country_code,
			DROP COLUMN recovery_phone_number;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Fails loudly if the backfill left anyone without an email.
	sqlx::query(
		r#"
		ALTER TABLE "user"
			ALTER COLUMN email TYPE CITEXT,
			ALTER COLUMN email SET NOT NULL,
			ADD CONSTRAINT user_uq_email UNIQUE(email);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Pending sign-ups are short-lived OTP rows keyed on the old username, so
	// there's nothing worth migrating — rebuild the table in the new shape.
	sqlx::query(
		r#"
		DROP TABLE user_to_sign_up;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE user_to_sign_up(
			email CITEXT NOT NULL,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,

			otp_hash TEXT NOT NULL,
			otp_expiry TIMESTAMPTZ NOT NULL,
			sign_up_attempts INTEGER NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_to_sign_up
			ADD CONSTRAINT user_to_sign_up_pk PRIMARY KEY(email);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			user_to_sign_up_idx_otp_expiry
		ON
			user_to_sign_up
		(otp_expiry);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			user_to_sign_up_idx_email_otp_expiry
		ON
			user_to_sign_up
		(email, otp_expiry);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// m010 created this as TEXT with a lower-case CHECK. It's matched against
	// `"user".email`, so it has to be CITEXT too or the two disagree on what
	// counts as the same address.
	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite
			DROP CONSTRAINT workspace_user_invite_chk_email_is_lower_case,
			ALTER COLUMN email TYPE CITEXT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE
			user_unverified_email,
			user_unverified_phone_number,
			user_email,
			user_phone_number,
			phone_number_country_code;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

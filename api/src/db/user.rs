use std::net::IpAddr;

use api_models::{
	models::user::{BasicUserInfo, UserPhoneNumber},
	utils::{ResourceType, Uuid},
};
use chrono::{DateTime, Utc};

use crate::{db::Workspace, query, query_as, Database};

pub struct User {
	pub id: Uuid,
	pub username: String,
	pub password: String,
	pub first_name: String,
	pub last_name: String,
	pub dob: Option<DateTime<Utc>>,
	pub bio: Option<String>,
	pub location: Option<String>,
	pub created: DateTime<Utc>,
	pub recovery_email_local: Option<String>,
	pub recovery_email_domain_id: Option<Uuid>,
	pub recovery_phone_country_code: Option<String>,
	pub recovery_phone_number: Option<String>,
	pub workspace_limit: i32,
	pub sign_up_coupon: Option<String>,
}

pub struct UserLogin {
	pub login_id: Uuid,
	/// Hashed refresh token
	pub refresh_token: String,
	pub token_expiry: DateTime<Utc>,
	pub user_id: Uuid,
	pub created: DateTime<Utc>,
	pub created_ip: IpAddr,
	pub created_location_latitude: f64,
	pub created_location_longitude: f64,
	pub created_country: String,
	pub created_region: String,
	pub created_city: String,
	pub created_timezone: String,
	pub last_login: DateTime<Utc>,
	pub last_activity: DateTime<Utc>,
	pub last_activity_ip: IpAddr,
	pub last_activity_location_latitude: f64,
	pub last_activity_location_longitude: f64,
	pub last_activity_country: String,
	pub last_activity_region: String,
	pub last_activity_city: String,
	pub last_activity_timezone: String,
	pub last_activity_user_agent: String,
}

pub struct PasswordResetRequest {
	pub user_id: Uuid,
	pub token: String,
	pub token_expiry: DateTime<Utc>,
}

pub struct PhoneCountryCode {
	pub country_code: String,
	pub phone_code: String,
	pub country_name: String,
}

pub struct PersonalEmailToBeVerified {
	pub local: String,
	pub domain_id: Uuid,
	pub user_id: Uuid,
	pub verification_token_hash: String,
	pub verification_token_expiry: DateTime<Utc>,
}

pub struct PhoneNumberToBeVerified {
	pub country_code: String,
	pub phone_number: String,
	pub user_id: Uuid,
	pub verification_token_hash: String,
	pub verification_token_expiry: DateTime<Utc>,
}

pub struct UserToSignUp {
	pub username: String,
	pub account_type: ResourceType,

	pub password: String,
	pub first_name: String,
	pub last_name: String,

	pub recovery_email_local: Option<String>,
	pub recovery_email_domain_id: Option<Uuid>,

	pub recovery_phone_country_code: Option<String>,
	pub recovery_phone_number: Option<String>,

	pub business_email_local: Option<String>,
	pub business_domain_name: Option<String>,
	pub business_name: Option<String>,

	pub otp_hash: String,
	pub otp_expiry: DateTime<Utc>,

	pub coupon_code: Option<String>,
}

pub struct CouponCode {
	pub code: String,
	pub credits: u32,
	pub expiry: Option<DateTime<Utc>>,
	pub uses_remaining: Option<u32>,
}

pub async fn initialize_users_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing user tables");
	query!(
		r#"
		CREATE TABLE coupon_code(
			code TEXT CONSTRAINT coupon_code_pk PRIMARY KEY,
			credits INTEGER NOT NULL CONSTRAINT coupon_code_chk_credits_positive
				CHECK(credits >= 0),
			expiry TIMESTAMPTZ,
			uses_remaining INTEGER CONSTRAINT
				coupon_code_chk_uses_remaining_positive CHECK(
					uses_remaining IS NULL OR uses_remaining > 0
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE "user"(
			id UUID CONSTRAINT user_pk PRIMARY KEY,
			username VARCHAR(100) NOT NULL
				CONSTRAINT user_uq_username UNIQUE
				CONSTRAINT user_chk_username_is_valid CHECK(
					/* Username is a-z, 0-9, _, cannot begin or end with a . or - */
					username ~ '^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$' AND
					username NOT LIKE '%..%' AND
					username NOT LIKE '%.-%' AND
					username NOT LIKE '%-.%'
				),
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			dob TIMESTAMPTZ DEFAULT NULL,
			bio VARCHAR(128) DEFAULT NULL,
			location VARCHAR(128) DEFAULT NULL,
			created TIMESTAMPTZ NOT NULL,
			/* Recovery options */
			recovery_email_local VARCHAR(64)
				CONSTRAINT user_chk_recovery_email_is_lower_case CHECK(
					recovery_email_local = LOWER(recovery_email_local)
				),
			recovery_email_domain_id UUID,
			recovery_phone_country_code CHAR(2)
				CONSTRAINT user_chk_recovery_phone_country_code_is_upper_case CHECK(
					recovery_phone_country_code = UPPER(recovery_phone_country_code)
				),
			recovery_phone_number VARCHAR(15),
			workspace_limit INTEGER NOT NULL,
			sign_up_coupon TEXT CONSTRAINT user_fk_sign_up_coupon REFERENCES
				coupon_code(code),

			CONSTRAINT user_uq_recovery_email_local_recovery_email_domain_id
				UNIQUE(recovery_email_local, recovery_email_domain_id),

			CONSTRAINT user_uq_recovery_phone_country_code_recovery_phone_number
				UNIQUE(recovery_phone_country_code, recovery_phone_number),

			CONSTRAINT user_chk_rcvry_eml_or_rcvry_phn_present CHECK(
				(
					recovery_email_local IS NOT NULL AND
					recovery_email_domain_id IS NOT NULL
				) OR
				(
					recovery_phone_country_code IS NOT NULL AND
					recovery_phone_number IS NOT NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_idx_created
		ON
			"user"
		(created);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_login(
			login_id UUID CONSTRAINT user_login_uq_login_id UNIQUE,
			refresh_token TEXT NOT NULL,
			token_expiry TIMESTAMPTZ NOT NULL,
			user_id UUID NOT NULL
				CONSTRAINT user_login_fk_user_id REFERENCES "user"(id),
			created TIMESTAMPTZ NOT NULL,
			created_ip INET NOT NULL,
			created_location GEOMETRY NOT NULL,
			created_country TEXT NOT NULL,
			created_region TEXT NOT NULL,
			created_city TEXT NOT NULL,
			created_timezone TEXT NOT NULL,
			last_login TIMESTAMPTZ NOT NULL,
			last_activity TIMESTAMPTZ NOT NULL,
			last_activity_ip INET NOT NULL,
			last_activity_location GEOMETRY NOT NULL,
			last_activity_country TEXT NOT NULL,
			last_activity_region TEXT NOT NULL,
			last_activity_city TEXT NOT NULL,
			last_activity_timezone TEXT NOT NULL,
			last_activity_user_agent TEXT NOT NULL,
			CONSTRAINT user_login_pk PRIMARY KEY(login_id, user_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_login_idx_user_id
		ON
			user_login
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE password_reset_request(
			user_id UUID
				CONSTRAINT password_reset_request_pk PRIMARY KEY
				CONSTRAINT password_reset_request_fk_user_id
					REFERENCES "user"(id),
			token TEXT NOT NULL,
			token_expiry TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_users_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up user tables initialization");
	query!(
		r#"
		CREATE TABLE personal_email(
			user_id UUID NOT NULL
				CONSTRAINT personal_email_fk_user_id REFERENCES "user"(id)
					DEFERRABLE INITIALLY IMMEDIATE,
			local VARCHAR(64) NOT NULL
				CONSTRAINT personal_email_chk_local_is_lower_case CHECK(
					local = LOWER(local)
				),
			domain_id UUID NOT NULL
				CONSTRAINT personal_email_fk_domain_id
					REFERENCES personal_domain(id),
			CONSTRAINT personal_email_pk PRIMARY KEY(local, domain_id),
			CONSTRAINT personal_email_uq_user_id_local_domain_id
				UNIQUE(user_id, local, domain_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			personal_email_idx_user_id
		ON
			personal_email
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE business_email(
			user_id UUID NOT NULL
				CONSTRAINT business_email_fk_user_id REFERENCES "user"(id),
			local VARCHAR(64) NOT NULL
				CONSTRAINT business_email_chk_local_is_lower_case CHECK(
					local = LOWER(local)
				),
			domain_id UUID NOT NULL
				CONSTRAINT business_email_fk_domain_id
					REFERENCES workspace_domain(id),
			CONSTRAINT business_email_pk PRIMARY KEY(local, domain_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			business_email_idx_user_id
		ON
			business_email
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE phone_number_country_code(
			country_code CHAR(2)
				CONSTRAINT phone_number_country_code_pk PRIMARY KEY
				CONSTRAINT phone_number_country_code_chk_country_code_is_upper_case CHECK(
					country_code = UPPER(country_code)
				),
			phone_code VARCHAR(5) NOT NULL,
			country_name VARCHAR(80) NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			phone_number_country_code_idx_phone_code
		ON
			phone_number_country_code
		(phone_code);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_phone_number(
			user_id UUID NOT NULL
				CONSTRAINT user_phone_number_fk_user_id REFERENCES "user"(id)
					DEFERRABLE INITIALLY IMMEDIATE,
			country_code CHAR(2) NOT NULL
				CONSTRAINT user_phone_number_fk_country_code
					REFERENCES phone_number_country_code(country_code)
				CONSTRAINT user_phone_number_chk_country_code_is_upper_case CHECK(
					country_code = UPPER(country_code)
				),
			number VARCHAR(15) NOT NULL
				CONSTRAINT user_phone_number_chk_number_valid CHECK(
					LENGTH(number) >= 7 AND
					LENGTH(number) <= 15 AND
					CAST(number AS BIGINT) > 0
				),
			CONSTRAINT user_phone_number_pk PRIMARY KEY(country_code, number),
			CONSTRAINT user_phone_number_uq_user_id_country_code_number
				UNIQUE(user_id, country_code, number)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_phone_number_idx_user_id
		ON
			user_phone_number
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_unverified_personal_email(
			local VARCHAR(64) NOT NULL
				CONSTRAINT user_unverified_personal_email_chk_local_is_lower_case CHECK(
					local = LOWER(local)
				),
			domain_id UUID NOT NULL
				CONSTRAINT user_unverified_personal_email_fk_domain_id
					REFERENCES personal_domain(id),
			user_id UUID NOT NULL
				CONSTRAINT user_unverified_personal_email_fk_user_id
					REFERENCES "user"(id),
			verification_token_hash TEXT NOT NULL,
			verification_token_expiry TIMESTAMPTZ NOT NULL,
			
			CONSTRAINT user_unverified_personal_email_pk
				PRIMARY KEY(local, domain_id),
			CONSTRAINT
				user_unverified_personal_email_uq_user_id_local_domain_id
				UNIQUE(user_id, local, domain_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_unverified_phone_number(
			country_code CHAR(2) NOT NULL
				CONSTRAINT user_unverified_phone_number_fk_country_code
					REFERENCES phone_number_country_code(country_code)
				CONSTRAINT user_unverified_phone_number_chk_country_code_is_upper_case CHECK(
					country_code = UPPER(country_code)
				),
			phone_number VARCHAR(15) NOT NULL,
			user_id UUID NOT NULL
				CONSTRAINT user_unverified_phone_number_fk_user_id
					REFERENCES "user"(id),
			verification_token_hash TEXT NOT NULL,
			verification_token_expiry TIMESTAMPTZ NOT NULL,

			CONSTRAINT user_univerified_phone_number_pk
				PRIMARY KEY(country_code, phone_number),
			CONSTRAINT
				user_univerified_phone_number_uq_country_code_phone_number
				UNIQUE(user_id, country_code, phone_number)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_to_sign_up(
			username VARCHAR(100) CONSTRAINT user_to_sign_up_pk PRIMARY KEY
				CONSTRAINT user_to_sign_up_chk_username_is_valid CHECK(
					/* Username is a-z, 0-9, _, cannot begin or end with a . or - */
					username ~ '^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$' AND
					username NOT LIKE '%..%' AND
					username NOT LIKE '%.-%' AND
					username NOT LIKE '%-.%'
				),
			account_type RESOURCE_OWNER_TYPE NOT NULL,

			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			
			/* Personal email address OR recovery email */
			recovery_email_local VARCHAR(64)
				CONSTRAINT user_to_sign_up_chk_recovery_email_is_lower_case CHECK(
					recovery_email_local = LOWER(recovery_email_local)
				),
			recovery_email_domain_id UUID
				CONSTRAINT user_to_sign_up_fk_recovery_email_domain_id
					REFERENCES personal_domain(id),

			recovery_phone_country_code CHAR(2)
				CONSTRAINT user_to_sign_up_fk_recovery_phone_country_code
					REFERENCES phone_number_country_code(country_code)
				CONSTRAINT user_to_sign_up_chk_recovery_phone_country_code_upper_case CHECK(
					recovery_phone_country_code = UPPER(recovery_phone_country_code)
				),
			recovery_phone_number VARCHAR(15)
				CONSTRAINT user_to_sign_up_chk_phone_number_valid CHECK(
					LENGTH(recovery_phone_number) >= 7 AND
					LENGTH(recovery_phone_number) <= 15 AND
					CAST(recovery_phone_number AS BIGINT) > 0
				),

			/* Workspace email address */
			business_email_local VARCHAR(64)
				CONSTRAINT
					user_to_sign_up_chk_business_email_local_is_lower_case
						CHECK(
							business_email_local = LOWER(business_email_local)
						),
			business_domain_name TEXT
				CONSTRAINT user_to_sign_up_chk_business_domain_name_is_valid
					CHECK(
						business_domain_name ~
							'^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$'
					),
			business_domain_tld TEXT
				CONSTRAINT
					user_to_sign_up_chk_business_domain_tld_is_length_valid
						CHECK(
							LENGTH(business_domain_tld) >= 2 AND
							LENGTH(business_domain_tld) <= 6
						)
				CONSTRAINT user_to_sign_up_chk_business_domain_tld_is_valid
					CHECK(
						business_domain_tld ~
							'^(([a-z0-9])|([a-z0-9][a-z0-9\-\.]*[a-z0-9]))$'
					)
				CONSTRAINT user_to_sign_up_fk_business_domain_tld
					REFERENCES domain_tld(tld),
			business_name VARCHAR(100)
				CONSTRAINT user_to_sign_up_chk_business_name_is_lower_case
					CHECK(business_name = LOWER(business_name)),
			otp_hash TEXT NOT NULL,
			otp_expiry TIMESTAMPTZ NOT NULL,
			coupon_code TEXT CONSTRAINT user_to_sign_up_fk_coupon_code
				REFERENCES coupon_code(code),

			CONSTRAINT user_to_sign_up_chk_max_domain_name_length CHECK(
				(LENGTH(business_domain_name) + LENGTH(business_domain_tld)) < 255
			),
			CONSTRAINT user_to_sign_up_chk_business_details_valid CHECK(
				(
					account_type = 'personal' AND
					(
						business_email_local IS NULL AND
						business_domain_name IS NULL AND
						business_domain_tld IS NULL AND
						business_name IS NULL
					)
				) OR
				(
					account_type = 'business' AND
					(
						business_email_local IS NOT NULL AND
						business_domain_name IS NOT NULL AND
						business_domain_tld IS NOT NULL AND
						business_name IS NOT NULL
					)
				)
			),
			CONSTRAINT user_to_sign_up_chk_recovery_details CHECK(
				(
					recovery_email_local IS NOT NULL AND
					recovery_email_domain_id IS NOT NULL AND
					recovery_phone_country_code IS NULL AND
					recovery_phone_number IS NULL
				) OR
				(
					recovery_email_local IS NULL AND
					recovery_email_domain_id IS NULL AND
					recovery_phone_country_code IS NOT NULL AND
					recovery_phone_number IS NOT NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_to_sign_up_idx_otp_expiry
		ON
			user_to_sign_up
		(otp_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_to_sign_up_idx_username_otp_expiry
		ON
			user_to_sign_up
		(username, otp_expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ADD CONSTRAINT user_fk_id_recovery_email_local_recovery_email_domain_id
		FOREIGN KEY(
			id,
			recovery_email_local,
			recovery_email_domain_id
		)
		REFERENCES personal_email(
			user_id,
			local,
			domain_id
		)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// add user id as a foreign key
	query!(
		r#"
		ALTER TABLE "user"
		ADD CONSTRAINT user_fk_id_recovery_phone_country_code_recovery_phone_number
		FOREIGN KEY(
			id,
			recovery_phone_country_code,
			recovery_phone_number
		)
		REFERENCES user_phone_number(
			user_id,
			country_code,
			number
		)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			phone_number_country_code(
				country_code,
				phone_code,
				country_name
			)
		VALUES
			($$AF$$, $$93$$, $$Afghanistan$$),
			($$AX$$, $$358$$, $$Aland Islands$$),
			($$AL$$, $$355$$, $$Albania$$),
			($$DZ$$, $$213$$, $$Algeria$$),
			($$AS$$, $$1684$$, $$American Samoa$$),
			($$AD$$, $$376$$, $$Andorra$$),
			($$AO$$, $$244$$, $$Angola$$),
			($$AI$$, $$1264$$, $$Anguilla$$),
			($$AQ$$, $$672$$, $$Antarctica$$),
			($$AG$$, $$1268$$, $$Antigua and Barbuda$$),
			($$AR$$, $$54$$, $$Argentina$$),
			($$AM$$, $$374$$, $$Armenia$$),
			($$AW$$, $$297$$, $$Aruba$$),
			($$AU$$, $$61$$, $$Australia$$),
			($$AT$$, $$43$$, $$Austria$$),
			($$AZ$$, $$994$$, $$Azerbaijan$$),
			($$BS$$, $$1242$$, $$Bahamas$$),
			($$BH$$, $$973$$, $$Bahrain$$),
			($$BD$$, $$880$$, $$Bangladesh$$),
			($$BB$$, $$1246$$, $$Barbados$$),
			($$BY$$, $$375$$, $$Belarus$$),
			($$BE$$, $$32$$, $$Belgium$$),
			($$BZ$$, $$501$$, $$Belize$$),
			($$BJ$$, $$229$$, $$Benin$$),
			($$BM$$, $$1441$$, $$Bermuda$$),
			($$BT$$, $$975$$, $$Bhutan$$),
			($$BO$$, $$591$$, $$Bolivia$$),
			($$BQ$$, $$599$$, $$Bonaire, Sint Eustatius and Saba$$),
			($$BA$$, $$387$$, $$Bosnia and Herzegovina$$),
			($$BW$$, $$267$$, $$Botswana$$),
			($$BV$$, $$55$$, $$Bouvet Island$$),
			($$BR$$, $$55$$, $$Brazil$$),
			($$IO$$, $$246$$, $$British Indian Ocean Territory$$),
			($$BN$$, $$673$$, $$Brunei Darussalam$$),
			($$BG$$, $$359$$, $$Bulgaria$$),
			($$BF$$, $$226$$, $$Burkina Faso$$),
			($$BI$$, $$257$$, $$Burundi$$),
			($$KH$$, $$855$$, $$Cambodia$$),
			($$CM$$, $$237$$, $$Cameroon$$),
			($$CA$$, $$1$$, $$Canada$$),
			($$CV$$, $$238$$, $$Cape Verde$$),
			($$KY$$, $$1345$$, $$Cayman Islands$$),
			($$CF$$, $$236$$, $$Central African Republic$$),
			($$TD$$, $$235$$, $$Chad$$),
			($$CL$$, $$56$$, $$Chile$$),
			($$CN$$, $$86$$, $$China$$),
			($$CX$$, $$61$$, $$Christmas Island$$),
			($$CC$$, $$672$$, $$Cocos (Keeling) Islands$$),
			($$CO$$, $$57$$, $$Colombia$$),
			($$KM$$, $$269$$, $$Comoros$$),
			($$CG$$, $$242$$, $$Congo$$),
			($$CD$$, $$242$$, $$Congo, Democratic Republic of the Congo$$),
			($$CK$$, $$682$$, $$Cook Islands$$),
			($$CR$$, $$506$$, $$Costa Rica$$),
			($$CI$$, $$225$$, $$Cote D'Ivoire$$),
			($$HR$$, $$385$$, $$Croatia$$),
			($$CU$$, $$53$$, $$Cuba$$),
			($$CW$$, $$599$$, $$Curacao$$),
			($$CY$$, $$357$$, $$Cyprus$$),
			($$CZ$$, $$420$$, $$Czech Republic$$),
			($$DK$$, $$45$$, $$Denmark$$),
			($$DJ$$, $$253$$, $$Djibouti$$),
			($$DM$$, $$1767$$, $$Dominica$$),
			($$DO$$, $$1809$$, $$Dominican Republic$$),
			($$EC$$, $$593$$, $$Ecuador$$),
			($$EG$$, $$20$$, $$Egypt$$),
			($$SV$$, $$503$$, $$El Salvador$$),
			($$GQ$$, $$240$$, $$Equatorial Guinea$$),
			($$ER$$, $$291$$, $$Eritrea$$),
			($$EE$$, $$372$$, $$Estonia$$),
			($$ET$$, $$251$$, $$Ethiopia$$),
			($$FK$$, $$500$$, $$Falkland Islands (Malvinas)$$),
			($$FO$$, $$298$$, $$Faroe Islands$$),
			($$FJ$$, $$679$$, $$Fiji$$),
			($$FI$$, $$358$$, $$Finland$$),
			($$FR$$, $$33$$, $$France$$),
			($$GF$$, $$594$$, $$French Guiana$$),
			($$PF$$, $$689$$, $$French Polynesia$$),
			($$TF$$, $$262$$, $$French Southern Territories$$),
			($$GA$$, $$241$$, $$Gabon$$),
			($$GM$$, $$220$$, $$Gambia$$),
			($$GE$$, $$995$$, $$Georgia$$),
			($$DE$$, $$49$$, $$Germany$$),
			($$GH$$, $$233$$, $$Ghana$$),
			($$GI$$, $$350$$, $$Gibraltar$$),
			($$GR$$, $$30$$, $$Greece$$),
			($$GL$$, $$299$$, $$Greenland$$),
			($$GD$$, $$1473$$, $$Grenada$$),
			($$GP$$, $$590$$, $$Guadeloupe$$),
			($$GU$$, $$1671$$, $$Guam$$),
			($$GT$$, $$502$$, $$Guatemala$$),
			($$GG$$, $$44$$, $$Guernsey$$),
			($$GN$$, $$224$$, $$Guinea$$),
			($$GW$$, $$245$$, $$Guinea-Bissau$$),
			($$GY$$, $$592$$, $$Guyana$$),
			($$HT$$, $$509$$, $$Haiti$$),
			($$HM$$, $$0$$, $$Heard Island and Mcdonald Islands$$),
			($$VA$$, $$39$$, $$Holy See (Vatican City State)$$),
			($$HN$$, $$504$$, $$Honduras$$),
			($$HK$$, $$852$$, $$Hong Kong$$),
			($$HU$$, $$36$$, $$Hungary$$),
			($$IS$$, $$354$$, $$Iceland$$),
			($$IN$$, $$91$$, $$India$$),
			($$ID$$, $$62$$, $$Indonesia$$),
			($$IR$$, $$98$$, $$Iran, Islamic Republic of$$),
			($$IQ$$, $$964$$, $$Iraq$$),
			($$IE$$, $$353$$, $$Ireland$$),
			($$IM$$, $$44$$, $$Isle of Man$$),
			($$IL$$, $$972$$, $$Israel$$),
			($$IT$$, $$39$$, $$Italy$$),
			($$JM$$, $$1876$$, $$Jamaica$$),
			($$JP$$, $$81$$, $$Japan$$),
			($$JE$$, $$44$$, $$Jersey$$),
			($$JO$$, $$962$$, $$Jordan$$),
			($$KZ$$, $$7$$, $$Kazakhstan$$),
			($$KE$$, $$254$$, $$Kenya$$),
			($$KI$$, $$686$$, $$Kiribati$$),
			($$KP$$, $$850$$, $$Korea, Democratic People's Republic of$$),
			($$KR$$, $$82$$, $$Korea, Republic of$$),
			($$XK$$, $$381$$, $$Kosovo$$),
			($$KW$$, $$965$$, $$Kuwait$$),
			($$KG$$, $$996$$, $$Kyrgyzstan$$),
			($$LA$$, $$856$$, $$Lao People's Democratic Republic$$),
			($$LV$$, $$371$$, $$Latvia$$),
			($$LB$$, $$961$$, $$Lebanon$$),
			($$LS$$, $$266$$, $$Lesotho$$),
			($$LR$$, $$231$$, $$Liberia$$),
			($$LY$$, $$218$$, $$Libyan Arab Jamahiriya$$),
			($$LI$$, $$423$$, $$Liechtenstein$$),
			($$LT$$, $$370$$, $$Lithuania$$),
			($$LU$$, $$352$$, $$Luxembourg$$),
			($$MO$$, $$853$$, $$Macao$$),
			($$MK$$, $$389$$, $$Macedonia, the Former Yugoslav Republic of$$),
			($$MG$$, $$261$$, $$Madagascar$$),
			($$MW$$, $$265$$, $$Malawi$$),
			($$MY$$, $$60$$, $$Malaysia$$),
			($$MV$$, $$960$$, $$Maldives$$),
			($$ML$$, $$223$$, $$Mali$$),
			($$MT$$, $$356$$, $$Malta$$),
			($$MH$$, $$692$$, $$Marshall Islands$$),
			($$MQ$$, $$596$$, $$Martinique$$),
			($$MR$$, $$222$$, $$Mauritania$$),
			($$MU$$, $$230$$, $$Mauritius$$),
			($$YT$$, $$269$$, $$Mayotte$$),
			($$MX$$, $$52$$, $$Mexico$$),
			($$FM$$, $$691$$, $$Micronesia, Federated States of$$),
			($$MD$$, $$373$$, $$Moldova, Republic of$$),
			($$MC$$, $$377$$, $$Monaco$$),
			($$MN$$, $$976$$, $$Mongolia$$),
			($$ME$$, $$382$$, $$Montenegro$$),
			($$MS$$, $$1664$$, $$Montserrat$$),
			($$MA$$, $$212$$, $$Morocco$$),
			($$MZ$$, $$258$$, $$Mozambique$$),
			($$MM$$, $$95$$, $$Myanmar$$),
			($$NA$$, $$264$$, $$Namibia$$),
			($$NR$$, $$674$$, $$Nauru$$),
			($$NP$$, $$977$$, $$Nepal$$),
			($$NL$$, $$31$$, $$Netherlands$$),
			($$AN$$, $$599$$, $$Netherlands Antilles$$),
			($$NC$$, $$687$$, $$New Caledonia$$),
			($$NZ$$, $$64$$, $$New Zealand$$),
			($$NI$$, $$505$$, $$Nicaragua$$),
			($$NE$$, $$227$$, $$Niger$$),
			($$NG$$, $$234$$, $$Nigeria$$),
			($$NU$$, $$683$$, $$Niue$$),
			($$NF$$, $$672$$, $$Norfolk Island$$),
			($$MP$$, $$1670$$, $$Northern Mariana Islands$$),
			($$NO$$, $$47$$, $$Norway$$),
			($$OM$$, $$968$$, $$Oman$$),
			($$PK$$, $$92$$, $$Pakistan$$),
			($$PW$$, $$680$$, $$Palau$$),
			($$PS$$, $$970$$, $$Palestinian Territory, Occupied$$),
			($$PA$$, $$507$$, $$Panama$$),
			($$PG$$, $$675$$, $$Papua New Guinea$$),
			($$PY$$, $$595$$, $$Paraguay$$),
			($$PE$$, $$51$$, $$Peru$$),
			($$PH$$, $$63$$, $$Philippines$$),
			($$PN$$, $$64$$, $$Pitcairn$$),
			($$PL$$, $$48$$, $$Poland$$),
			($$PT$$, $$351$$, $$Portugal$$),
			($$PR$$, $$1787$$, $$Puerto Rico$$),
			($$QA$$, $$974$$, $$Qatar$$),
			($$RE$$, $$262$$, $$Reunion$$),
			($$RO$$, $$40$$, $$Romania$$),
			($$RU$$, $$70$$, $$Russian Federation$$),
			($$RW$$, $$250$$, $$Rwanda$$),
			($$BL$$, $$590$$, $$Saint Barthelemy$$),
			($$SH$$, $$290$$, $$Saint Helena$$),
			($$KN$$, $$1869$$, $$Saint Kitts and Nevis$$),
			($$LC$$, $$1758$$, $$Saint Lucia$$),
			($$MF$$, $$590$$, $$Saint Martin$$),
			($$PM$$, $$508$$, $$Saint Pierre and Miquelon$$),
			($$VC$$, $$1784$$, $$Saint Vincent and the Grenadines$$),
			($$WS$$, $$684$$, $$Samoa$$),
			($$SM$$, $$378$$, $$San Marino$$),
			($$ST$$, $$239$$, $$Sao Tome and Principe$$),
			($$SA$$, $$966$$, $$Saudi Arabia$$),
			($$SN$$, $$221$$, $$Senegal$$),
			($$RS$$, $$381$$, $$Serbia$$),
			($$CS$$, $$381$$, $$Serbia and Montenegro$$),
			($$SC$$, $$248$$, $$Seychelles$$),
			($$SL$$, $$232$$, $$Sierra Leone$$),
			($$SG$$, $$65$$, $$Singapore$$),
			($$SX$$, $$1$$, $$Sint Maarten$$),
			($$SK$$, $$421$$, $$Slovakia$$),
			($$SI$$, $$386$$, $$Slovenia$$),
			($$SB$$, $$677$$, $$Solomon Islands$$),
			($$SO$$, $$252$$, $$Somalia$$),
			($$ZA$$, $$27$$, $$South Africa$$),
			($$GS$$, $$500$$, $$South Georgia and the South Sandwich Islands$$),
			($$SS$$, $$211$$, $$South Sudan$$),
			($$ES$$, $$34$$, $$Spain$$),
			($$LK$$, $$94$$, $$Sri Lanka$$),
			($$SD$$, $$249$$, $$Sudan$$),
			($$SR$$, $$597$$, $$Suriname$$),
			($$SJ$$, $$47$$, $$Svalbard and Jan Mayen$$),
			($$SZ$$, $$268$$, $$Swaziland$$),
			($$SE$$, $$46$$, $$Sweden$$),
			($$CH$$, $$41$$, $$Switzerland$$),
			($$SY$$, $$963$$, $$Syrian Arab Republic$$),
			($$TW$$, $$886$$, $$Taiwan, Province of China$$),
			($$TJ$$, $$992$$, $$Tajikistan$$),
			($$TZ$$, $$255$$, $$Tanzania, United Republic of$$),
			($$TH$$, $$66$$, $$Thailand$$),
			($$TL$$, $$670$$, $$Timor-Leste$$),
			($$TG$$, $$228$$, $$Togo$$),
			($$TK$$, $$690$$, $$Tokelau$$),
			($$TO$$, $$676$$, $$Tonga$$),
			($$TT$$, $$1868$$, $$Trinidad and Tobago$$),
			($$TN$$, $$216$$, $$Tunisia$$),
			($$TR$$, $$90$$, $$Turkey$$),
			($$TM$$, $$7370$$, $$Turkmenistan$$),
			($$TC$$, $$1649$$, $$Turks and Caicos Islands$$),
			($$TV$$, $$688$$, $$Tuvalu$$),
			($$UG$$, $$256$$, $$Uganda$$),
			($$UA$$, $$380$$, $$Ukraine$$),
			($$AE$$, $$971$$, $$United Arab Emirates$$),
			($$GB$$, $$44$$, $$United Kingdom$$),
			($$US$$, $$1$$, $$United States$$),
			($$UM$$, $$1$$, $$United States Minor Outlying Islands$$),
			($$UY$$, $$598$$, $$Uruguay$$),
			($$UZ$$, $$998$$, $$Uzbekistan$$),
			($$VU$$, $$678$$, $$Vanuatu$$),
			($$VE$$, $$58$$, $$Venezuela$$),
			($$VN$$, $$84$$, $$Viet Nam$$),
			($$VG$$, $$1284$$, $$Virgin Islands, British$$),
			($$VI$$, $$1340$$, $$Virgin Islands, U.s.$$),
			($$WF$$, $$681$$, $$Wallis and Futuna$$),
			($$EH$$, $$212$$, $$Western Sahara$$),
			($$YE$$, $$967$$, $$Yemen$$),
			($$ZM$$, $$260$$, $$Zambia$$),
			($$ZW$$, $$263$$, $$Zimbabwe$$);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_user_by_username_email_or_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		LEFT JOIN
			personal_email
		ON
			personal_email.user_id = "user".id
		LEFT JOIN
			business_email
		ON
			business_email.user_id = "user".id
		LEFT JOIN
			domain
		ON
			domain.id = personal_email.domain_id OR
			domain.id = business_email.domain_id
		LEFT JOIN
			user_phone_number
		ON
			user_phone_number.user_id = "user".id
		LEFT JOIN
			phone_number_country_code
		ON
			phone_number_country_code.country_code = user_phone_number.country_code
		WHERE
			"user".username = $1 OR
			CONCAT(
				personal_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1 OR
			CONCAT(
				business_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1 OR
			CONCAT(
				'+',
				phone_number_country_code.phone_code,
				user_phone_number.number
			) = $1;
		"#,
		user_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_by_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	email: &str,
) -> Result<Option<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		LEFT JOIN
			personal_email
		ON
			personal_email.user_id = "user".id
		LEFT JOIN
			business_email
		ON
			business_email.user_id = "user".id
		LEFT JOIN
			domain
		ON
			domain.id = personal_email.domain_id OR
			domain.id = business_email.domain_id
		WHERE
			CONCAT(
				personal_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1 OR
			CONCAT(
				business_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1;
		"#,
		email
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_by_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
) -> Result<Option<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		INNER JOIN
			user_phone_number
		ON
			"user".id = user_phone_number.user_id
		WHERE
			user_phone_number.country_code = $1 AND
			user_phone_number.number = $2;
		"#,
		country_code,
		phone_number
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_by_username(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
) -> Result<Option<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		WHERE
			username = $1;
		"#,
		username
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_by_user_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Option<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn generate_new_user_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				"user"
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn get_god_user_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Option<Uuid>, sqlx::Error> {
	let uuid = query!(
		r#"
		SELECT
			id as "id: Uuid"
		FROM
			"user"
		ORDER BY
			created
		LIMIT 1;
		"#
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| row.id);

	Ok(uuid)
}

pub async fn set_personal_user_to_be_signed_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),

	email_local: Option<&str>,
	email_domain_id: Option<&Uuid>,
	recovery_phone_country_code: Option<&str>,
	recovery_phone_number: Option<&str>,

	otp_hash: &str,
	otp_expiry: &DateTime<Utc>,

	coupon_code: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_to_sign_up(
				username,
				account_type,
				password,
				first_name,
				last_name,
				recovery_email_local,
				recovery_email_domain_id,
				recovery_phone_country_code,
				recovery_phone_number,
				business_email_local,
				business_domain_name,
				business_domain_tld,
				business_name,
				otp_hash,
				otp_expiry,
				coupon_code
			)
		VALUES
			(
				$1,
				'personal',
				
				$2,
				$3,
				$4,
				
				$5,
				$6,
				
				$7,
				$8,
				
				NULL,
				NULL,
				NULL,
				NULL,
				
				$9,
				$10,

				$11
			)
		ON CONFLICT(username) DO UPDATE SET
			account_type = 'personal',

			password = EXCLUDED.password,
			first_name = EXCLUDED.first_name,
			last_name = EXCLUDED.last_name,
			
			recovery_email_local = EXCLUDED.recovery_email_local,
			recovery_email_domain_id = EXCLUDED.recovery_email_domain_id,

			recovery_phone_country_code = EXCLUDED.recovery_phone_country_code,
			recovery_phone_number = EXCLUDED.recovery_phone_number,
			
			business_email_local = NULL,
			business_domain_name = NULL,
			business_name = NULL,
			
			otp_hash = EXCLUDED.otp_hash,
			otp_expiry = EXCLUDED.otp_expiry,
			
			coupon_code = EXCLUDED.coupon_code;
		"#,
		username,
		password,
		first_name,
		last_name,
		email_local,
		email_domain_id as _,
		recovery_phone_country_code,
		recovery_phone_number,
		otp_hash,
		otp_expiry as _,
		coupon_code,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_business_user_to_be_signed_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),

	recovery_email_local: Option<&str>,
	recovery_email_domain_id: Option<&Uuid>,
	recovery_phone_country_code: Option<&str>,
	recovery_phone_number: Option<&str>,

	business_email_local: &str,
	business_domain_name: &str,
	business_domain_tld: &str,
	business_name: &str,

	otp_hash: &str,
	otp_expiry: &DateTime<Utc>,

	coupon_code: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_to_sign_up(
				username,
				account_type,
				password,
				first_name,
				last_name,
				recovery_email_local,
				recovery_email_domain_id,
				recovery_phone_country_code,
				recovery_phone_number,
				business_email_local,
				business_domain_name,
				business_domain_tld,
				business_name,
				otp_hash,
				otp_expiry,
				coupon_code
			)
		VALUES
			(
				$1,
				'business',
				
				$2,
				$3,
				$4,
				
				$5,
				$6,
				
				$7,
				$8,
				
				$9,
				$10,
				$11,
				$12,

				$13,
				$14,

				$15
			)
		ON CONFLICT(username) DO UPDATE SET
			account_type = 'business',

			password = EXCLUDED.password,
			first_name = EXCLUDED.first_name,
			last_name = EXCLUDED.last_name,
			
			recovery_email_local = EXCLUDED.recovery_email_local,
			recovery_email_domain_id = EXCLUDED.recovery_email_domain_id,

			recovery_phone_country_code = EXCLUDED.recovery_phone_country_code,
			recovery_phone_number = EXCLUDED.recovery_phone_number,
			
			business_email_local = EXCLUDED.business_email_local,
			business_domain_name = EXCLUDED.business_domain_name,
			business_domain_tld = EXCLUDED.business_domain_tld,
			business_name = EXCLUDED.business_name,
			
			otp_hash = EXCLUDED.otp_hash,
			otp_expiry = EXCLUDED.otp_expiry,
			
			coupon_code = EXCLUDED.coupon_code;
		"#,
		username,
		password,
		first_name,
		last_name,
		recovery_email_local,
		recovery_email_domain_id as _,
		recovery_phone_country_code,
		recovery_phone_number,
		business_email_local,
		business_domain_name,
		business_domain_tld,
		business_name,
		otp_hash,
		otp_expiry as _,
		coupon_code,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_user_to_sign_up_by_username(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			username,
			account_type as "account_type: _",
			password,
			first_name,
			last_name,
			recovery_email_local,
			recovery_email_domain_id as "recovery_email_domain_id: _",
			recovery_phone_country_code,
			recovery_phone_number,
			business_email_local,
			CASE
				WHEN business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(business_domain_name, '.', business_domain_tld)
			END as "business_domain_name: _",
			business_name,
			otp_hash,
			otp_expiry,
			coupon_code
		FROM
			user_to_sign_up
		WHERE
			username = $1;
		"#,
		username
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_to_sign_up_by_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			username,
			account_type as "account_type: _",
			password,
			first_name,
			last_name,
			recovery_email_local,
			recovery_email_domain_id as "recovery_email_domain_id: _",
			recovery_phone_country_code,
			recovery_phone_number,
			business_email_local,
			CASE
				WHEN business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(business_domain_name, '.', business_domain_tld)
			END as "business_domain_name: _",
			business_name,
			otp_hash,
			otp_expiry,
			coupon_code
		FROM
			user_to_sign_up
		WHERE
			recovery_phone_country_code = $1 AND
			recovery_phone_number = $2;
		"#,
		country_code,
		phone_number
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_to_sign_up_by_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	email: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			user_to_sign_up.username,
			user_to_sign_up.account_type as "account_type: _",
			user_to_sign_up.password,
			user_to_sign_up.first_name,
			user_to_sign_up.last_name,
			user_to_sign_up.recovery_email_local,
			user_to_sign_up.recovery_email_domain_id as "recovery_email_domain_id: _",
			user_to_sign_up.recovery_phone_country_code,
			user_to_sign_up.recovery_phone_number,
			user_to_sign_up.business_email_local,
			CASE
				WHEN user_to_sign_up.business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(
						user_to_sign_up.business_domain_name,
						'.',
						user_to_sign_up.business_domain_tld
					)
			END as "business_domain_name: _",
			user_to_sign_up.business_name,
			user_to_sign_up.otp_hash,
			user_to_sign_up.otp_expiry,
			user_to_sign_up.coupon_code
		FROM
			user_to_sign_up
		INNER JOIN
			domain
		ON
			domain.id = user_to_sign_up.recovery_email_domain_id
		WHERE
			CONCAT(
				user_to_sign_up.recovery_email_local,
				'@',
				domain.name,
				'.',
				domain.tld
			) = $1;
		"#,
		email
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_to_sign_up_by_business_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	business_name: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			username,
			account_type as "account_type: _",
			password,
			first_name,
			last_name,
			recovery_email_local,
			recovery_email_domain_id as "recovery_email_domain_id: _",
			recovery_phone_country_code,
			recovery_phone_number,
			business_email_local,
			CASE
				WHEN business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(business_domain_name, '.', business_domain_tld)
			END as "business_domain_name: _",
			business_name,
			otp_hash,
			otp_expiry,
			coupon_code
		FROM
			user_to_sign_up
		WHERE
			business_name = $1;
		"#,
		business_name
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_to_sign_up_by_business_domain_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	business_domain_name: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	query_as!(
		UserToSignUp,
		r#"
		SELECT
			username,
			account_type as "account_type: _",
			password,
			first_name,
			last_name,
			recovery_email_local,
			recovery_email_domain_id as "recovery_email_domain_id: _",
			recovery_phone_country_code,
			recovery_phone_number,
			business_email_local,
			CASE
				WHEN business_domain_name IS NULL THEN
					NULL
				ELSE
					CONCAT(business_domain_name, '.', business_domain_tld)
			END as "business_domain_name: _",
			business_name,
			otp_hash,
			otp_expiry,
			coupon_code
		FROM
			user_to_sign_up
		WHERE
			business_domain_name = $1;
		"#,
		business_domain_name
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_user_to_sign_up_with_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
	verification_token: &str,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_to_sign_up
		SET
			otp_hash = $1,
			otp_expiry = $2
		WHERE
			username = $3;
		"#,
		verification_token,
		token_expiry as _,
		username
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	email_local: &str,
	domain_id: &Uuid,
	user_id: &Uuid,
	verification_token: &str,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_unverified_personal_email(
				local,
				domain_id,
				user_id,
				verification_token_hash,
				verification_token_expiry
			)
		VALUES
			($1, $2, $3, $4, $5)
		ON CONFLICT(local, domain_id) DO UPDATE SET
			user_id = EXCLUDED.user_id,
			verification_token_hash = EXCLUDED.verification_token_hash,
			verification_token_expiry = EXCLUDED.verification_token_expiry;
		"#,
		email_local,
		domain_id as _,
		user_id as _,
		verification_token,
		token_expiry as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn add_phone_number_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
	user_id: &Uuid,
	verification_token: &str,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_unverified_phone_number(
				country_code,
				phone_number,
				user_id,
				verification_token_hash,
				verification_token_expiry
			)
		VALUES
			($1, $2, $3, $4, $5)
		ON CONFLICT(country_code, phone_number) DO UPDATE SET
			user_id = EXCLUDED.user_id,
			verification_token_hash = EXCLUDED.verification_token_hash,
			verification_token_expiry = EXCLUDED.verification_token_expiry;
		"#,
		country_code,
		phone_number,
		user_id as _,
		verification_token,
		token_expiry as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_personal_email_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email: &str,
) -> Result<Option<PersonalEmailToBeVerified>, sqlx::Error> {
	query_as!(
		PersonalEmailToBeVerified,
		r#"
		SELECT
			user_unverified_personal_email.local,
			user_unverified_personal_email.domain_id as "domain_id: _",
			user_unverified_personal_email.user_id as "user_id: _",
			user_unverified_personal_email.verification_token_hash,
			user_unverified_personal_email.verification_token_expiry
		FROM
			user_unverified_personal_email
		INNER JOIN
			domain
		ON
			domain.id = user_unverified_personal_email.domain_id
		WHERE
			user_id = $1 AND
			CONCAT(local, '@', domain.name, '.', domain.tld) = $2;
		"#,
		user_id as _,
		email
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_personal_email_to_be_verified_by_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	email: &str,
) -> Result<Option<PersonalEmailToBeVerified>, sqlx::Error> {
	query_as!(
		PersonalEmailToBeVerified,
		r#"
		SELECT
			user_unverified_personal_email.local,
			user_unverified_personal_email.domain_id as "domain_id: _",
			user_unverified_personal_email.user_id as "user_id: _",
			user_unverified_personal_email.verification_token_hash,
			user_unverified_personal_email.verification_token_expiry
		FROM
			user_unverified_personal_email
		INNER JOIN
			domain
		ON
			domain.id = user_unverified_personal_email.domain_id
		WHERE
			CONCAT(local, '@', domain.name, '.', domain.tld) = $1;
		"#,
		email
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn delete_personal_email_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_unverified_personal_email
		WHERE
			user_id = $1 AND
			local = $2 AND
			domain_id = $3;
		"#,
		user_id as _,
		email_local,
		domain_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_phone_number_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	country_code: &str,
	phone_number: &str,
) -> Result<Option<PhoneNumberToBeVerified>, sqlx::Error> {
	query_as!(
		PhoneNumberToBeVerified,
		r#"
		SELECT
			user_unverified_phone_number.country_code,
			user_unverified_phone_number.phone_number,
			user_unverified_phone_number.user_id as "user_id: _",
			user_unverified_phone_number.verification_token_hash,
			user_unverified_phone_number.verification_token_expiry
		FROM
			user_unverified_phone_number
		INNER JOIN
			phone_number_country_code
		ON
			user_unverified_phone_number.country_code = phone_number_country_code.country_code
		WHERE
			user_id = $1 AND
			user_unverified_phone_number.country_code = $2 AND
			user_unverified_phone_number.phone_number = $3;
		"#,
		user_id as _,
		country_code,
		phone_number
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_phone_number_to_be_verified_by_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
	phone_number: &str,
) -> Result<Option<PhoneNumberToBeVerified>, sqlx::Error> {
	query_as!(
		PhoneNumberToBeVerified,
		r#"
		SELECT
			country_code,
			phone_number,
			user_id as "user_id: _",
			verification_token_hash,
			verification_token_expiry
		FROM
			user_unverified_phone_number
		WHERE
			country_code = $1 AND
			phone_number = $2;
		"#,
		country_code,
		phone_number
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn delete_phone_number_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	country_code: &str,
	phone_number: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_unverified_phone_number
		WHERE
			user_id = $1 AND
			country_code = $2 AND
			phone_number = $3;
		"#,
		user_id as _,
		country_code,
		phone_number
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_personal_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_email(
				user_id,
				local,
				domain_id
			)
		VALUES
			($1, $2, $3);
		"#,
		user_id as _,
		email_local,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_business_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			business_email(
				user_id,
				local,
				domain_id
			)
		VALUES
			($1, $2, $3);
		"#,
		user_id as _,
		email_local,
		domain_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_user_to_be_signed_up(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_to_sign_up
		WHERE
			username = $1;
		"#,
		username,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),
	created: &DateTime<Utc>,

	recovery_email_local: Option<&str>,
	recovery_email_domain_id: Option<&Uuid>,

	recovery_phone_country_code: Option<&str>,
	recovery_phone_number: Option<&str>,

	workspace_limit: i32,
	sign_up_coupon: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			"user"(
				id,
				username,
				password,
				first_name,
				last_name,
				dob,
				bio,
				location,
				created,

				recovery_email_local,
				recovery_email_domain_id,

				recovery_phone_country_code,
				recovery_phone_number,

				workspace_limit,

				sign_up_coupon
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				NULL,
				NULL,
				NULL,
				$6,
				
				$7,
				$8,
				
				$9,
				$10,

				$11,

				$12
			);
		"#,
		user_id as _,
		username,
		password,
		first_name,
		last_name,
		created as _,
		recovery_email_local,
		recovery_email_domain_id as _,
		recovery_phone_country_code,
		recovery_phone_number,
		workspace_limit,
		sign_up_coupon,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

#[allow(clippy::too_many_arguments)]
pub async fn add_user_login(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	refresh_token: &str,
	token_expiry: &DateTime<Utc>,
	user_id: &Uuid,
	created: &DateTime<Utc>,
	created_ip: &IpAddr,
	created_location_latitude: f64,
	created_location_longitude: f64,
	created_country: &str,
	created_region: &str,
	created_city: &str,
	created_timezone: &str,
	last_login: &DateTime<Utc>,
	last_activity: &DateTime<Utc>,
	last_activity_ip: &IpAddr,
	last_activity_location_latitude: f64,
	last_activity_location_longitude: f64,
	last_activity_country: &str,
	last_activity_region: &str,
	last_activity_city: &str,
	last_activity_timezone: &str,
	last_activity_user_agent: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_login(
				login_id,
				refresh_token, 
				token_expiry, 
				user_id, 
				created,
				created_ip,
				created_location,
				created_country,
				created_region,
				created_city,
				created_timezone,
				last_login, 
				last_activity,
				last_activity_ip,
				last_activity_location,
				last_activity_country,
				last_activity_region,
				last_activity_city,
				last_activity_timezone,
				last_activity_user_agent
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				ST_SetSRID(POINT($7, $8)::GEOMETRY, 4326),
				$9,
				$10,
				$11,
				$12,
				$13,
				$14,
				$15,
				ST_SetSRID(POINT($16, $17)::GEOMETRY, 4326),
				$18,
				$19,
				$20,
				$21,
				$22
			);
		"#,
		login_id as _,
		refresh_token,
		token_expiry as _,
		user_id as _,
		created as _,
		created_ip as _,
		created_location_latitude as _,
		created_location_longitude as _,
		created_country,
		created_region,
		created_city,
		created_timezone,
		last_login as _,
		last_activity as _,
		last_activity_ip as _,
		last_activity_location_latitude as _,
		last_activity_location_longitude as _,
		last_activity_country,
		last_activity_region,
		last_activity_city,
		last_activity_timezone,
		last_activity_user_agent,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_user_login(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
) -> Result<Option<UserLogin>, sqlx::Error> {
	query_as!(
		UserLogin,
		r#"
		SELECT
			login_id as "login_id: _",
			refresh_token,
			token_expiry,
			user_id as "user_id: _",
			created,
			created_ip as "created_ip: _",
			ST_Y(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_latitude!: _",
			ST_X(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_longitude!: _",
			created_country,
			created_region,
			created_city,
			created_timezone,
			last_login,
			last_activity,
			last_activity_ip as "last_activity_ip: _",
			ST_Y(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_latitude!: _",
			ST_X(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_longitude!: _",
			last_activity_country,
			last_activity_region,
			last_activity_city,
			last_activity_timezone,
			last_activity_user_agent
		FROM
			user_login
		WHERE
			login_id = $1;
		"#,
		login_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_login_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	user_id: &Uuid,
) -> Result<Option<UserLogin>, sqlx::Error> {
	query_as!(
		UserLogin,
		r#"
		SELECT
			login_id as "login_id: _",
			refresh_token,
			token_expiry,
			user_id as "user_id: _",
			created,
			created_ip as "created_ip: _",
			ST_Y(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_latitude!: _",
			ST_X(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_longitude!: _",
			created_country,
			created_region,
			created_city,
			created_timezone,
			last_login,
			last_activity,
			last_activity_ip as "last_activity_ip: _",
			ST_Y(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_latitude!: _",
			ST_X(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_longitude!: _",
			last_activity_country,
			last_activity_region,
			last_activity_city,
			last_activity_timezone,
			last_activity_user_agent
		FROM
			user_login
		WHERE
			login_id = $1 AND
			user_id = $2;
		"#,
		login_id as _,
		user_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_user_login_last_activity_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	last_activity: &DateTime<Utc>,
	last_activity_ip: &IpAddr,
	last_activity_location_latitude: f64,
	last_activity_location_longitude: f64,
	last_activity_country: &str,
	last_activity_region: &str,
	last_activity_city: &str,
	last_activity_timezone: &str,
	last_activity_user_agent: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_login
		SET
			last_activity = $1,
			last_activity_ip = $2,
			last_activity_location = ST_SetSRID(POINT($3, $4)::GEOMETRY, 4326),
			last_activity_country = $5,
			last_activity_region = $6,
			last_activity_city = $7,
			last_activity_timezone = $8,
			last_activity_user_agent = $9
		WHERE
			login_id = $10;
		"#,
		last_activity,
		last_activity_ip as _,
		last_activity_location_latitude,
		last_activity_location_longitude,
		last_activity_country,
		last_activity_region,
		last_activity_city,
		last_activity_timezone,
		last_activity_user_agent,
		login_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn generate_new_login_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				login_id
			FROM
				user_login
			WHERE
				login_id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn get_all_logins_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<UserLogin>, sqlx::Error> {
	query_as!(
		UserLogin,
		r#"
		SELECT
			login_id as "login_id: _",
			refresh_token,
			token_expiry,
			user_id as "user_id: _",
			created as "created: _",
			created_ip as "created_ip: _",
			ST_Y(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_latitude!: _",
			ST_X(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_longitude!: _",
			created_country,
			created_region,
			created_city,
			created_timezone,
			last_login,
			last_activity,
			last_activity_ip as "last_activity_ip: _",
			ST_Y(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_latitude!: _",
			ST_X(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_longitude!: _",
			last_activity_country,
			last_activity_region,
			last_activity_city,
			last_activity_timezone,
			last_activity_user_agent
		FROM
			user_login
		WHERE
			user_id = $1;
		"#,
		user_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_login_for_user_with_refresh_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	refresh_token: &str,
) -> Result<Option<UserLogin>, sqlx::Error> {
	query_as!(
		UserLogin,
		r#"
		SELECT
			login_id as "login_id: _",
			refresh_token,
			token_expiry,
			user_id as "user_id: _",
			created as "created: _",
			created_ip as "created_ip: _",
			ST_Y(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_latitude!: _",
			ST_X(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_longitude!: _",
			created_country,
			created_region,
			created_city,
			created_timezone,
			last_login,
			last_activity,
			last_activity_ip as "last_activity_ip: _",
			ST_Y(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_latitude!: _",
			ST_X(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_longitude!: _",
			last_activity_country,
			last_activity_region,
			last_activity_city,
			last_activity_timezone,
			last_activity_user_agent
		FROM
			user_login
		WHERE
			user_id = $1 AND
			refresh_token = $2;
		"#,
		user_id as _,
		refresh_token,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn delete_user_login_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	user_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_login
		SET
			token_expiry = TO_TIMESTAMP(0)
		WHERE
			login_id = $1 AND
			user_id = $2;
		"#,
		login_id as _,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_login_expiry(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	last_activity: &DateTime<Utc>,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_login
		SET
			token_expiry = $1,
			last_activity = $2
		WHERE
			login_id = $3;
		"#,
		token_expiry as _,
		last_activity as _,
		login_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_user_data(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	first_name: Option<&str>,
	last_name: Option<&str>,
	dob: Option<&DateTime<Utc>>,
	bio: Option<&str>,
	location: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			"user"
		SET
			first_name = COALESCE($1, first_name),
			last_name = COALESCE($2, last_name),
			dob = COALESCE($3, dob),
			bio = COALESCE($4, bio),
			location = COALESCE($5, location)
		WHERE
			id = $6;
		"#,
		first_name,
		last_name,
		dob as _,
		bio,
		location,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_user_password(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	password: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			"user"
		SET
			password = $1
		WHERE
			id = $2;
		"#,
		password,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_recovery_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			"user"
		SET
			recovery_email_local = $1,
			recovery_email_domain_id = $2
		WHERE
			id = $3;
		"#,
		email_local,
		domain_id as _,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_recovery_phone_number_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	country_code: &str,
	phone_number: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			"user"
		SET
			recovery_phone_country_code = $1,
			recovery_phone_number = $2
		WHERE
			id = $3;
		"#,
		country_code,
		phone_number,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_password_reset_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	token_hash: &str,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			password_reset_request(
				user_id,
				token,
				token_expiry
			)
		VALUES
			($1, $2, $3)
		ON CONFLICT(user_id) DO UPDATE SET
			token = EXCLUDED.token,
			token_expiry = EXCLUDED.token_expiry;
		"#,
		user_id as _,
		token_hash,
		token_expiry as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_password_reset_request_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Option<PasswordResetRequest>, sqlx::Error> {
	query_as!(
		PasswordResetRequest,
		r#"
		SELECT
			user_id as "user_id: _",
			token,
			token_expiry
		FROM
			password_reset_request
		WHERE
			user_id = $1;
		"#,
		user_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn delete_password_reset_request_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			password_reset_request
		WHERE
			user_id = $1;
		"#,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_workspaces_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<Workspace>, sqlx::Error> {
	query_as!(
		Workspace,
		r#"
		SELECT DISTINCT
			workspace.id as "id: _",
			workspace.name::TEXT as "name!: _",
			workspace.super_admin_id as "super_admin_id: _",
			workspace.active,
			workspace.alert_emails,
			workspace.payment_type as "payment_type: _",
			workspace.default_payment_method_id as "default_payment_method_id: _",
			workspace.deployment_limit,
			workspace.static_site_limit,
			workspace.database_limit,
			workspace.managed_url_limit,
			workspace.secret_limit,
			workspace.domain_limit,
			workspace.docker_repository_storage_limit,
			workspace.stripe_customer_id,
			workspace.address_id as "address_id: _",
			workspace.amount_due
		FROM
			workspace
		LEFT JOIN
			workspace_user
		ON
			workspace.id = workspace_user.workspace_id
		WHERE
			(
				workspace.super_admin_id = $1 OR
				workspace_user.user_id = $1
			) AND
			workspace.deleted IS NULL;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_workspaces_owned_by_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<Workspace>, sqlx::Error> {
	query_as!(
		Workspace,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			super_admin_id as "super_admin_id: _",
			active,
			alert_emails,
			payment_type as "payment_type: _",
			default_payment_method_id as "default_payment_method_id: _",
			deployment_limit,
			static_site_limit,
			database_limit,
			managed_url_limit,
			secret_limit,
			domain_limit,
			docker_repository_storage_limit,
			stripe_customer_id,
			address_id as "address_id: _",
			amount_due
		FROM
			workspace
		WHERE
			super_admin_id = $1 AND
			deleted IS NULL;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_phone_country_by_country_code(
	connection: &mut <Database as sqlx::Database>::Connection,
	country_code: &str,
) -> Result<Option<PhoneCountryCode>, sqlx::Error> {
	query_as!(
		PhoneCountryCode,
		r#"
		SELECT
			*
		FROM
			phone_number_country_code
		WHERE
			country_code = $1;
		"#,
		country_code
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn add_phone_number_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	phone_country_code: &str,
	phone_number: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_phone_number(
				user_id,
				country_code,
				number
			)
		VALUES
			($1, $2, $3);
		"#,
		user_id as _,
		phone_country_code,
		phone_number
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_personal_emails_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<String>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			CONCAT(
				personal_email.local,
				'@',
				domain.name,
				'.',
				domain.tld
			) as "email!: String"
		FROM
			personal_email
		INNER JOIN
			domain
		ON
			personal_email.domain_id = domain.id
		WHERE
			personal_email.user_id = $1;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.email)
	.collect();

	Ok(rows)
}

pub async fn get_personal_email_count_for_domain_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	domain_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	query!(
		r#"
		SELECT
			COUNT(personal_email.domain_id) as "count!: i64"
		FROM
			personal_email
		WHERE
			personal_email.domain_id = $1;
		"#,
		domain_id as _,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.count as u64)
}

pub async fn get_phone_numbers_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<UserPhoneNumber>, sqlx::Error> {
	query_as!(
		UserPhoneNumber,
		r#"
		SELECT
			country_code,
			number as "phone_number"
		FROM
			user_phone_number
		WHERE
			user_id = $1;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_recovery_phone_number_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Option<UserPhoneNumber>, sqlx::Error> {
	query_as!(
		UserPhoneNumber,
		r#"
		SELECT
			"user".recovery_phone_country_code as "country_code!",
			"user".recovery_phone_number as "phone_number!"
		FROM
			"user"
		WHERE
			"user".id = $1 AND
			"user".recovery_phone_number IS NOT NULL AND
			"user".recovery_phone_country_code IS NOT NULL;
		"#,
		user_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_recovery_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
	query!(
		r#"
		SELECT
			CONCAT(
				"user".recovery_email_local,
				'@',
				domain.name,
				'.',
				domain.tld
			) as "email!: String"
		FROM
			"user"
		INNER JOIN
			domain
		ON
			"user".recovery_email_domain_id = domain.id
		WHERE
			"user".id = $1;
		"#,
		user_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|row| row.map(|row| row.email))
}

pub async fn delete_personal_email_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	email_local: &str,
	domain_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			personal_email
		WHERE
			user_id = $1 AND
			local = $2 AND
			domain_id = $3;
		"#,
		user_id as _,
		email_local,
		domain_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_phone_number_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	country_code: &str,
	phone_number: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_phone_number
		WHERE
			user_id = $1 AND
			country_code = $2 AND
			number = $3;
		"#,
		user_id as _,
		country_code,
		phone_number
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn search_for_users(
	connection: &mut <Database as sqlx::Database>::Connection,
	query: &str,
) -> Result<Vec<BasicUserInfo>, sqlx::Error> {
	query_as!(
		BasicUserInfo,
		r#"
		SELECT DISTINCT
			"user".id as "id!: Uuid",
			"user".username as "username!",
			"user".first_name as "first_name!",
			"user".last_name as "last_name!",
			"user".bio as "bio",
			"user".location as "location"
		FROM
			"user"
		LEFT JOIN
			personal_email
		ON
			"user".id = personal_email.user_id
		LEFT JOIN
			business_email
		ON
			"user".id = personal_email.user_id
		LEFT JOIN
			domain
		ON
			domain.id = personal_email.domain_id OR
			domain.id = business_email.domain_id
		LEFT JOIN
			user_phone_number
		ON
			"user".id = user_phone_number.user_id
		LEFT JOIN
			phone_number_country_code
		ON
			user_phone_number.country_code = phone_number_country_code.country_code
		WHERE
			(
				CASE
					WHEN LOWER("user".username) = LOWER($1) THEN 100 /* Exact username */
					WHEN COALESCE(
						CASE
							WHEN personal_email.local IS NOT NULL THEN
								LOWER(CONCAT(
									personal_email.local,
									'@',
									domain.name,
									domain.tld
								))
							ELSE NULL
						END,
						CASE
							WHEN business_email.local IS NOT NULL THEN
								LOWER(CONCAT(
									business_email.local,
									'@',
									domain.name,
									domain.tld
								))
							ELSE NULL
						END
					) = LOWER($1) THEN 100 /* Exact email */
					WHEN CONCAT(
						'+',
						phone_number_country_code.phone_code,
						user_phone_number.number
					) = $1 THEN 100 /* Exact phone number */
					WHEN user_phone_number.number = $1 THEN 90 /* Just the phone alone */
					WHEN LOWER(CONCAT(
						"user".first_name,
						' ',
						"user".last_name
					)) = LOWER($1) THEN 90 /* firstName lastName */
					WHEN LOWER(CONCAT(
						"user".last_name,
						' ',
						"user".first_name
					)) = LOWER($1) THEN 90 /* lastName firstName */
					WHEN LOWER("user".first_name) = LOWER($1) THEN 80 /* Only first name */
					WHEN LOWER("user".last_name) = LOWER($1) THEN 80 /* Only last name */

					/* If you search for a part of their username */
					WHEN STARTS_WITH(
						LOWER("user".username),
						SUBSTR(LOWER($1), 0, LENGTH("user".username))
					) THEN (
						70 + (
							LENGTH($1)::REAL / LENGTH("user".username)::REAL
						) * 20
					)

					/* If you search for a part of their name */
					WHEN STARTS_WITH(
						LOWER(CONCAT("user".first_name, ' ', "user".last_name)),
						SUBSTR(
							LOWER($1),
							0,
							LENGTH(CONCAT("user".first_name, ' ', "user".last_name))
						)
					) THEN (
						70 + (
							LENGTH($1)::REAL / LENGTH(CONCAT("user".first_name, ' ', "user".last_name))::REAL
						) * 20
					)

					/* If you search for a part of their name reversed */
					WHEN STARTS_WITH(
						LOWER(CONCAT("user".last_name, ' ', "user".first_name)),
						SUBSTR(
							LOWER($1),
							0,
							LENGTH(CONCAT("user".last_name, ' ', "user".first_name))
						)
					) THEN (
						70 + (
							LENGTH($1)::REAL / LENGTH(CONCAT("user".last_name, ' ', "user".first_name))::REAL
						) * 20
					)

					ELSE 0
				END
			) > 0
		ORDER BY
			1 DESC
		LIMIT 10;
		"#,
		query
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_sign_up_coupon_by_code(
	connection: &mut <Database as sqlx::Database>::Connection,
	coupon_code: &str,
) -> Result<Option<CouponCode>, sqlx::Error> {
	let row = query!(
		r#"
		SELECT
			*
		FROM
			coupon_code
		WHERE
			code = $1;
		"#,
		coupon_code,
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| CouponCode {
		code: row.code,
		credits: row.credits as u32,
		uses_remaining: row.uses_remaining.map(|uses| uses as u32),
		expiry: row.expiry,
	});

	Ok(row)
}

pub async fn update_coupon_code_uses_remaining(
	connection: &mut <Database as sqlx::Database>::Connection,
	coupon_code: &str,
	uses_remaining: u32,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			coupon_code
		SET
			uses_remaining = $1
		WHERE
			code = $2;
		"#,
		uses_remaining as i32,
		coupon_code,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

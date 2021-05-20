use sqlx::Transaction;
use uuid::Uuid;

use crate::{
	models::db_mapping::{
		Organisation,
		PasswordResetRequest,
		PersonalEmailToBeVerified,
		PhoneCountryCode,
		User,
		UserLogin,
		UserToSignUp,
	},
	query,
	query_as,
	Database,
};

pub async fn initialize_users_pre(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing user tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS "user"(
			id BYTEA
				CONSTRAINT user_pk PRIMARY KEY,
			username VARCHAR(100) NOT NULL
				CONSTRAINT user_uk_username UNIQUE,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			dob BIGINT DEFAULT NULL
				CONSTRAINT user_chk_dob_unsigned CHECK(dob >= 0),
			bio VARCHAR(128) DEFAULT NULL,
			location VARCHAR(128) DEFAULT NULL,
			created BIGINT NOT NULL
				CONSTRAINT user_chk_created_unsigned CHECK(created >= 0),
			/* Recovery options */
			backup_email_local VARCHAR(64),
			backup_email_domain_id BYTEA,
			backup_phone_country_code CHAR(2),
			backup_phone_number VARCHAR(15),

			CONSTRAINT user_uk_bckp_eml_lcl_bckp_eml_dmn_id
				UNIQUE(backup_email_local, backup_email_domain_id),

			CONSTRAINT user_uk_bckp_phn_cntry_cd_bckp_phn_nmbr
				UNIQUE(backup_phone_country_code, backup_phone_number),

			CONSTRAINT user_chk_bckp_eml_or_bckp_phn_present CHECK(
				(
					backup_email_local IS NOT NULL AND
					backup_email_domain_id IS NOT NULL
				) OR
				(
					backup_phone_country_code IS NOT NULL AND
					backup_phone_number IS NOT NULL
				)
			)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_login(
			login_id BYTEA
				CONSTRAINT user_login_pk PRIMARY KEY,
			refresh_token TEXT NOT NULL,
			token_expiry BIGINT NOT NULL
				CONSTRAINT user_login_chk_token_expiry_unsigned
					CHECK(token_expiry >= 0),
			user_id BYTEA NOT NULL
				CONSTRAINT user_login_fk_user_id REFERENCES "user"(id),
			last_login BIGINT NOT NULL
				CONSTRAINT user_login_chk_last_login_unsigned
					CHECK(last_login >= 0),
			last_activity BIGINT NOT NULL
				CONSTRAINT user_login_chk_last_activity_unsigned
					CHECK(last_activity >= 0)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS password_reset_request(
			user_id BYTEA
				CONSTRAINT password_reset_request_pk PRIMARY KEY
				CONSTRAINT password_reset_request_fk_user_id
					REFERENCES "user"(id),
			token TEXT NOT NULL,
			token_expiry BIGINT NOT NULL
				CONSTRAINT password_reset_request_token_expiry_ck_unsigned
					CHECK(token_expiry >= 0)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_users_post(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS personal_email(
			user_id BYTEA NOT NULL
				CONSTRAINT personal_email_fk_user_id REFERENCES "user"(id)
					DEFERRABLE INITIALLY IMMEDIATE,
			local VARCHAR(64) NOT NULL,
			domain_id BYTEA NOT NULL
				CONSTRAINT personal_email_fk_domain_id
					REFERENCES personal_domain(id),
			CONSTRAINT personal_email_pk PRIMARY KEY(local, domain_id),
			CONSTRAINT personal_email_uq_user_id_local_domain_id
				UNIQUE(user_id, local, domain_id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation_email(
			user_id BYTEA NOT NULL
				CONSTRAINT organisation_email_fk_user_id REFERENCES "user"(id),
			local VARCHAR(64) NOT NULL,
			domain_id BYTEA NOT NULL
				CONSTRAINT organisation_email_fk_domain_id
					REFERENCES organisation_domain(id),
			CONSTRAINT organisation_email_pk PRIMARY KEY(local, domain_id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS phone_number_country_code(
			country_code CHAR(2)
				CONSTRAINT phone_number_country_code_pk PRIMARY KEY,
			phone_code VARCHAR(5) NOT NULL,
			country_name VARCHAR(80) NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_phone_number(
			user_id BYTEA NOT NULL
				CONSTRAINT user_phone_number_fk_user_id REFERENCES "user"(id)
					DEFERRABLE INITIALLY IMMEDIATE,
			country_code CHAR(2) NOT NULL
				CONSTRAINT user_phone_number_fk_country_code
					REFERENCES phone_number_country_code(country_code),
			number VARCHAR(15) NOT NULL
				CONSTRAINT user_phone_number_chk_number_valid CHECK(
					LENGTH(number) >= 7 AND
					LENGTH(number) <= 15 AND
					CAST(number AS INTEGER) > 0
				),
			CONSTRAINT user_phone_number_pk PRIMARY KEY(country_code, number),
			CONSTRAINT user_phone_number_uq_user_id_country_code_number
				UNIQUE(user_id, country_code, number)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_unverified_personal_email(
			local VARCHAR(64) NOT NULL,
			domain_id BYTEA NOT NULL
				CONSTRAINT user_unverified_personal_email_fk_domain_id
					REFERENCES personal_domain(id),
			user_id BYTEA NOT NULL
				CONSTRAINT user_unverified_personal_email_fk_user_id
					REFERENCES "user"(id),
			verification_token_hash TEXT NOT NULL,
			verification_token_expiry BIGINT NOT NULL
				CONSTRAINT
					user_unverified_personal_email_chk_token_expiry_unsigned
					CHECK(verification_token_expiry >= 0),
			
			CONSTRAINT user_unverified_personal_email_pk
				PRIMARY KEY(local, domain_id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_to_sign_up(
			username VARCHAR(100) CONSTRAINT user_to_sign_up_pk PRIMARY KEY,
			account_type RESOURCE_OWNER_TYPE NOT NULL,

			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			
			/* Personal email address OR backup email */
			backup_email_local VARCHAR(64),
			backup_email_domain_id BYTEA
				CONSTRAINT user_to_sign_up_fk_backup_email_domain_id
					REFERENCES personal_domain(id),

			backup_phone_country_code CHAR(2)
				CONSTRAINT user_to_sign_up_fk_backup_phone_country_code
					REFERENCES phone_number_country_code(country_code),
			backup_phone_number VARCHAR(15)
				CONSTRAINT user_to_sign_up_chk_phone_number_valid CHECK(
					LENGTH(backup_phone_number) >= 7 AND
					LENGTH(backup_phone_number) <= 15 AND
					CAST(backup_phone_number AS INTEGER) > 0
				),

			/* Organisation email address */
			org_email_local VARCHAR(64),
			org_domain_name VARCHAR(100),
			organisation_name VARCHAR(100),

			otp_hash TEXT NOT NULL,
			otp_expiry BIGINT NOT NULL
				CONSTRAINT user_to_sign_up_chk_expiry_unsigned
					CHECK(otp_expiry >= 0),

			CONSTRAINT user_to_sign_up_chk_org_details_valid CHECK(
				(
					account_type = 'personal' AND
					(
						org_email_local IS NULL AND
						org_domain_name IS NULL AND
						organisation_name IS NULL
					)
				) OR
				(
					account_type = 'organisation' AND
					(
						org_email_local IS NOT NULL AND
						org_domain_name IS NOT NULL AND
						organisation_name IS NOT NULL
					)
				)
			),
			CONSTRAINT user_to_sign_up_chk_backup_details CHECK(
				(
					backup_email_local IS NOT NULL AND
					backup_email_domain_id IS NOT NULL AND
					backup_phone_country_code IS NULL AND
					backup_phone_number IS NULL
				) OR
				(
					backup_email_local IS NULL AND
					backup_email_domain_id IS NULL AND
					backup_phone_country_code IS NOT NULL AND
					backup_phone_number IS NOT NULL
				)
			)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ADD CONSTRAINT user_fk_id_backup_email_local_backup_email_domain_id
		FOREIGN KEY (
			id,
			backup_email_local,
			backup_email_domain_id
		)
		REFERENCES personal_email (
			user_id,
			local,
			domain_id
		)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// add user id as a foreign key
	query!(
		r#"
		ALTER TABLE "user"
		ADD CONSTRAINT user_fk_id_backup_phone_country_code_backup_phone_number
		FOREIGN KEY (
			id,
			backup_phone_country_code,
			backup_phone_number
		)
		REFERENCES user_phone_number (
			user_id,
			country_code,
			number
		)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		INSERT INTO
			phone_number_country_code
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
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn get_user_by_username_or_email(
	connection: &mut Transaction<'_, Database>,
	user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			user.*
		FROM
			user
		LEFT JOIN
			personal_email
		ON
			personal_email.user_id = user.id
		LEFT JOIN
			organisation_email
		ON
			organisation_email.user_id = user.id
		LEFT JOIN
			domain
		ON
			domain.id = personal_email.domain_id OR
			domain.id = organisation_email.domain_id
		WHERE
			user.username = ? OR
			CONCAT(personal_email.local, '@', domain.name) = ? OR
			CONCAT(organisation_email.local, '@', domain.name) = ?;
		"#,
		user_id,
		user_id,
		user_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_user_by_email(
	connection: &mut Transaction<'_, Database>,
	email: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			user.*
		FROM
			user
		LEFT JOIN
			personal_email
		ON
			personal_email.user_id = user.id
		LEFT JOIN
			organisation_email
		ON
			organisation_email.user_id = user.id
		LEFT JOIN
			domain
		ON
			domain.id = personal_email.domain_id OR
			domain.id = organisation_email.domain_id
		WHERE
			CONCAT(personal_email.local, '@', domain.name) = ? OR
			CONCAT(organisation_email.local, '@', domain.name) = ?;
		"#,
		email,
		email
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_user_by_phone_number(
	connection: &mut Transaction<'_, Database>,
	phone_number: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			user.*
		FROM
			user
		INNER JOIN
			user_phone_number
		ON
			user.id = user_phone_number.user_id
		INNER JOIN
			phone_number_country_code
		ON
			user_phone_number.country_code = phone_number_country_code.country_code
		WHERE
			CONCAT(
				'+',
				phone_number_country_code.phone_code,
				user_phone_number.number
			) = ?;
		"#,
		phone_number
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_user_by_username(
	connection: &mut Transaction<'_, Database>,
	username: &str,
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			username = ?;
		"#,
		username
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_user_by_user_id(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
) -> Result<Option<User>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			id = ?;
		"#,
		user_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn generate_new_user_id(
	connection: &mut Transaction<'_, Database>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut rows = query_as!(
		User,
		r#"
		SELECT
			*
		FROM
			user
		WHERE
			id = ?;
		"#,
		uuid.as_bytes().as_ref()
	)
	.fetch_all(&mut *connection)
	.await?;

	while !rows.is_empty() {
		uuid = Uuid::new_v4();
		rows = query_as!(
			User,
			r#"
			SELECT
				*
			FROM
				user
			WHERE
				id = ?;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?;
	}

	Ok(uuid)
}

pub async fn get_god_user_id(
	connection: &mut Transaction<'_, Database>,
) -> Result<Option<Uuid>, sqlx::Error> {
	let rows = query_as!(
		User,
		r#"
		SELECT
			*
		FROM
			user
		ORDER BY
			created
		DESC
		LIMIT 1;
		"#
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();
	let id = Uuid::from_slice(row.id.as_ref())
		.expect("unable to unwrap UUID from Vec<u8>");

	Ok(Some(id))
}

pub async fn set_personal_user_to_be_signed_up(
	connection: &mut Transaction<'_, Database>,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),

	email_local: Option<&str>,
	email_domain_id: Option<&[u8]>,
	backup_phone_country_code: Option<&str>,
	backup_phone_number: Option<&str>,

	otp_hash: &str,
	otp_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_to_sign_up
		VALUES
			(
				?,
				'personal',
				
				?,
				?,
				?,
				
				?,
				?,
				
				?,
				?,
				
				NULL,
				NULL,
				NULL,
				
				?,
				?
			)
		ON DUPLICATE KEY UPDATE
			account_type = 'personal',

			password = ?,
			first_name = ?,
			last_name = ?,
			
			backup_email_local = ?,
			backup_email_domain_id = ?,

			backup_phone_country_code = ?,
			backup_phone_number = ?,
			
			org_email_local = NULL,
			org_domain_name = NULL,
			organisation_name = NULL,
			
			otp_hash = ?,
			otp_expiry = ?;
		"#,
		username,
		password,
		first_name,
		last_name,
		email_local,
		email_domain_id,
		backup_phone_country_code,
		backup_phone_number,
		otp_hash,
		otp_expiry,
		// On duplicate key
		password,
		first_name,
		last_name,
		email_local,
		email_domain_id,
		backup_phone_country_code,
		backup_phone_number,
		otp_hash,
		otp_expiry
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn set_organisation_user_to_be_signed_up(
	connection: &mut Transaction<'_, Database>,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),

	backup_email_local: Option<&str>,
	backup_email_domain_id: Option<&[u8]>,
	backup_phone_country_code: Option<&str>,
	backup_phone_number: Option<&str>,

	org_email_local: &str,
	org_domain_name: &str,
	organisation_name: &str,

	otp_hash: &str,
	otp_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_to_sign_up
		VALUES
			(
				?,
				'organisation',
				
				?,
				?,
				?,
				
				?,
				?,
				
				?,
				?,
				
				?,
				?,
				?,
				
				?,
				?
			)
		ON DUPLICATE KEY UPDATE
			account_type = 'organisation',

			password = ?,
			first_name = ?,
			last_name = ?,
			
			backup_email_local = ?,
			backup_email_domain_id = ?,

			backup_phone_country_code = ?,
			backup_phone_number = ?,
			
			org_email_local = ?,
			org_domain_name = ?,
			organisation_name = ?,
			
			otp_hash = ?,
			otp_expiry = ?;
		"#,
		username,
		password,
		first_name,
		last_name,
		backup_email_local,
		backup_email_domain_id,
		backup_phone_country_code,
		backup_phone_number,
		org_email_local,
		org_domain_name,
		organisation_name,
		otp_hash,
		otp_expiry,
		// On duplicate key
		password,
		first_name,
		last_name,
		backup_email_local,
		backup_email_domain_id,
		backup_phone_country_code,
		backup_phone_number,
		org_email_local,
		org_domain_name,
		organisation_name,
		otp_hash,
		otp_expiry
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_user_to_sign_up_by_username(
	connection: &mut Transaction<'_, Database>,
	username: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	let rows = query_as!(
		UserToSignUp,
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			username = ?;
		"#,
		username
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	Ok(rows.into_iter().next())
}

pub async fn get_user_to_sign_up_by_organisation_name(
	connection: &mut Transaction<'_, Database>,
	organisation_name: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	let rows = query_as!(
		UserToSignUp,
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			organisation_name = ?;
		"#,
		organisation_name
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	Ok(rows.into_iter().next())
}

pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut Transaction<'_, Database>,
	email_local: &str,
	domain_id: &[u8],
	user_id: &[u8],
	verification_token: &str,
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_unverified_personal_email
		VALUES
			(?, ?, ?, ?, ?)
		ON DUPLICATE KEY UPDATE
			user_id = ?,
			verification_token_hash = ?,
			verification_token_expiry = ?;
		"#,
		email_local,
		domain_id,
		user_id,
		verification_token,
		token_expiry,
		user_id,
		verification_token,
		token_expiry
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_personal_email_to_be_verified_for_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	email: &str,
) -> Result<Option<PersonalEmailToBeVerified>, sqlx::Error> {
	let rows = query_as!(
		PersonalEmailToBeVerified,
		r#"
		SELECT
			user_unverified_personal_email.*
		FROM
			user_unverified_personal_email
		INNER JOIN
			domain
		ON
			domain.id = user_unverified_personal_email.domain_id
		WHERE
			user_id = ? AND
			CONCAT(local, '@', domain.name) = ?;
		"#,
		user_id,
		email
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn add_personal_email_for_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	email_local: &str,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_email
		VALUES
			(?, ?, ?);
		"#,
		user_id,
		email_local,
		domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn create_orphaned_personal_email(
	connection: &mut Transaction<'_, Database>,
	email_local: &str,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_email
		VALUES
			(NULL, ?, ?);
		"#,
		email_local,
		domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn set_user_for_personal_email(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	email_local: &str,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			personal_email
		SET
			user_id = ?
		WHERE
			local = ? AND
			domain_id = ?;
		"#,
		user_id,
		email_local,
		domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_organisation_email_for_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	email_local: &str,
	domain_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			organisation_email
		VALUES
			(?, ?, ?);
		"#,
		user_id,
		email_local,
		domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn delete_user_to_be_signed_up(
	connection: &mut Transaction<'_, Database>,
	username: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_to_sign_up
		WHERE
			username = ?;
		"#,
		username,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn create_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),
	created: u64,

	backup_email_local: Option<&str>,
	backup_email_domain_id: Option<&[u8]>,

	backup_phone_country_code: Option<&str>,
	backup_phone_number: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user
		VALUES
			(
				?,
				?,
				?,
				?,
				?,
				NULL,
				NULL,
				NULL,
				?,
				
				?,
				?,
				
				?,
				?
			);
		"#,
		user_id,
		username,
		password,
		first_name,
		last_name,
		created,
		backup_email_local,
		backup_email_domain_id,
		backup_phone_country_code,
		backup_phone_number
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_user_login(
	connection: &mut Transaction<'_, Database>,
	login_id: &[u8],
	refresh_token: &str,
	token_expiry: u64,
	user_id: &[u8],
	last_login: u64,
	last_activity: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_login
		VALUES
			(?, ?, ?, ?, ?, ?);
		"#,
		login_id,
		refresh_token,
		token_expiry,
		user_id,
		last_login,
		last_activity
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_user_login(
	connection: &mut Transaction<'_, Database>,
	login_id: &[u8],
) -> Result<Option<UserLogin>, sqlx::Error> {
	let rows = query_as!(
		UserLogin,
		r#"
		SELECT
			*
		FROM
			user_login
		WHERE
			login_id = ?;
		"#,
		login_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn generate_new_login_id(
	connection: &mut Transaction<'_, Database>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut rows = query_as!(
		UserLogin,
		r#"
		SELECT
			*
		FROM
			user_login
		WHERE
			login_id = ?;
		"#,
		uuid.as_bytes().as_ref()
	)
	.fetch_all(&mut *connection)
	.await?;

	while !rows.is_empty() {
		uuid = Uuid::new_v4();
		rows = query_as!(
			UserLogin,
			r#"
			SELECT
				*
			FROM
				user_login
			WHERE
				login_id = ?;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?;
	}

	Ok(uuid)
}

pub async fn get_all_logins_for_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
) -> Result<Vec<UserLogin>, sqlx::Error> {
	query_as!(
		UserLogin,
		r#"
		SELECT
			*
		FROM
			user_login
		WHERE
			user_id = ?;
		"#,
		user_id
	)
	.fetch_all(connection)
	.await
}

pub async fn delete_user_login_by_id(
	connection: &mut Transaction<'_, Database>,
	login_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_login
		WHERE
			login_id = ?;
		"#,
		login_id,
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn set_login_expiry(
	connection: &mut Transaction<'_, Database>,
	login_id: &[u8],
	last_activity: u64,
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_login
		SET
			token_expiry = ?,
			last_activity = ?
		WHERE
			login_id = ?;
		"#,
		token_expiry,
		last_activity,
		login_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn update_user_data(
	connection: &mut Transaction<'_, Database>,
	first_name: Option<&str>,
	last_name: Option<&str>,
	dob: Option<&str>,
	bio: Option<&str>,
	location: Option<&str>,
) -> Result<(), sqlx::Error> {
	let params = [
		(first_name, "first_name"),
		(last_name, "last_name"),
		(dob, "dob"),
		(bio, "bio"),
		(location, "location"),
	];

	let param_updates = params
		.iter()
		.filter_map(|(param, name)| {
			if param.is_none() {
				None
			} else {
				Some(format!("{} = ?", name))
			}
		})
		.collect::<Vec<String>>()
		.join(", ");

	let query_string = format!("UPDATE user SET {};", param_updates);
	let mut query = sqlx::query(&query_string);
	for (param, _) in params.iter() {
		if let Some(value) = param {
			query = query.bind(value);
		}
	}
	query.execute(connection).await?;

	Ok(())
}

pub async fn update_user_password(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	password: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user
		SET
			password = ?
		WHERE
			id = ?;
		"#,
		password,
		user_id,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_password_reset_request(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	token_hash: &str,
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			password_reset_request
		VALUES
			(?, ?, ?)
		ON DUPLICATE KEY UPDATE
			token = ?,
			token_expiry = ?;
		"#,
		user_id,
		token_hash,
		token_expiry,
		token_hash,
		token_expiry,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_password_reset_request_for_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
) -> Result<Option<PasswordResetRequest>, sqlx::Error> {
	let rows = query_as!(
		PasswordResetRequest,
		r#"
		SELECT
			*
		FROM
			password_reset_request
		WHERE
			user_id = ?;
		"#,
		user_id,
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn delete_password_reset_request_for_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			password_reset_request
		WHERE
			user_id = ?;
		"#,
		user_id,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_all_organisations_for_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
) -> Result<Vec<Organisation>, sqlx::Error> {
	let organisations = query_as!(
		Organisation,
		r#"
		SELECT DISTINCT
			organisation.id,
			organisation.name,
			organisation.super_admin_id,
			organisation.active as `active: bool`,
			organisation.created
		FROM
			organisation
		LEFT JOIN
			organisation_user
		ON
			organisation.id = organisation_user.organisation_id
		WHERE
			organisation.super_admin_id = ? OR
			organisation_user.user_id = ?;
		"#,
		user_id,
		user_id
	)
	.fetch_all(connection)
	.await?;

	Ok(organisations)
}

pub async fn get_phone_country_by_country_code(
	transaction: &mut Transaction<'_, Database>,
	country_code: &str,
) -> Result<Option<PhoneCountryCode>, sqlx::Error> {
	let rows = query_as!(
		PhoneCountryCode,
		r#"
		SELECT
			*
		FROM
			phone_number_country_code
		WHERE
			country_code = ?;
		"#,
		country_code
	)
	.fetch_all(transaction)
	.await?;

	Ok(rows.into_iter().next())
}

#[allow(dead_code)]
pub async fn add_phone_number_for_user(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	phone_country_code: &str,
	phone_number: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_phone_number
		VALUES
			(?, ?, ?);
		"#,
		user_id,
		phone_country_code,
		phone_number
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn create_orphaned_phone_number(
	connection: &mut Transaction<'_, Database>,
	phone_country_code: &str,
	phone_number: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_phone_number
		VALUES
			(NULL, ?, ?);
		"#,
		phone_country_code,
		phone_number
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn set_user_for_phone_number(
	connection: &mut Transaction<'_, Database>,
	user_id: &[u8],
	phone_country_code: &str,
	phone_number: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_phone_number
		SET
			user_id = ?
		WHERE
			country_code = ? AND
			number = ?;
		"#,
		user_id,
		phone_country_code,
		phone_number
	)
	.execute(connection)
	.await
	.map(|_| ())
}

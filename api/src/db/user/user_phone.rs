use api_models::{models::user::UserPhoneNumber, utils::Uuid};
use chrono::{DateTime, Utc};

use super::User;
use crate::{query, query_as, Database};

pub struct PhoneCountryCode {
	pub country_code: String,
	pub phone_code: String,
	pub country_name: String,
}

pub struct PhoneNumberToBeVerified {
	pub country_code: String,
	pub phone_number: String,
	pub user_id: Uuid,
	pub verification_token_hash: String,
	pub verification_token_expiry: DateTime<Utc>,
}

pub async fn initialize_user_phone_pre(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}

pub async fn initialize_user_phone_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
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
			"user".sign_up_coupon,
			"user".mfa_secret
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

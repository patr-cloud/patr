use sqlx::{MySql, Transaction};
use uuid::Uuid;

use crate::{
	models::db_mapping::{
		Organisation,
		PasswordResetRequest,
		PersonalEmailToBeVerified,
		User,
		UserEmailAddress,
		UserEmailAddressSignUp,
		UserLogin,
		UserToSignUp,
	},
	query,
	query_as,
	utils::{self, constants::AccountType},
};

pub async fn initialize_users_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing user tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user (
			id BINARY(16) PRIMARY KEY,
			username VARCHAR(100) UNIQUE NOT NULL,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			dob BIGINT UNSIGNED DEFAULT NULL,
			bio VARCHAR(128) DEFAULT NULL,
			location VARCHAR(128) DEFAULT NULL,
			created BIGINT UNSIGNED NOT NULL,
			backup_email_local VARCHAR(64),
			backup_email_domain_id BINARY(16),
			backup_country_code VARCHAR(2),
			backup_phone_number VARCHAR(15),
			CONSTRAINT CHECK (
				(
					backup_email_local IS NOT NULL 
					AND
					backup_email_domain_id IS NOT NULL
				)
				OR
				(
					backup_phone_number IS NOT NULL
					AND
					backup_phone_number_country_code IS NOT NULL
					AND 
					backup_country_code IS NOT NULL
				)
				/*Foreign key added later */
			)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_login (
			login_id BINARY(16) PRIMARY KEY,
			refresh_token TEXT NOT NULL,
			token_expiry BIGINT UNSIGNED NOT NULL,
			user_id BINARY(16) NOT NULL,
			last_login BIGINT UNSIGNED NOT NULL,
			last_activity BIGINT UNSIGNED NOT NULL,
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS password_reset_request (
			user_id BINARY(16) PRIMARY KEY,
			token TEXT NOT NULL,
			token_expiry BIGINT UNSIGNED NOT NULL,
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_users_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS personal_email (
			user_id BINARY(16),
			email_local VARCHAR(64),
			domain_id BINARY(16),
			PRIMARY KEY(email_local, domain_id),
			UNIQUE(user_id,email_local,domain_id),
			FOREIGN KEY(user_id) REFERENCES user(id),
			FOREIGN KEY(domain_id) REFERENCES personal_domain(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// more details can be added here in the organisation table
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation_email (
			user_id BINARY(16),
			email_local VARCHAR(54),
			domain_id BINARY(16),
			PRIMARY KEY(email_local, domain_id),
			FOREIGN KEY(user_id) REFERENCES user(id),
			FOREIGN KEY(domain_id) REFERENCES organisation_domain(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// table for storing country codes, might prove more helpful in analytics
	// since i am also adding country name
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS phone_number_country_code (
			phone_code VARCHAR(5) NOT NULL,
			country_code VARCHAR(2) NOT NULL,
			country_name VARCHAR(80) NOT NULL,
			PRIMARY KEY(phone_code, country_code)
		);	
	"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		INSERT INTO 
			phone_number_country_code
		VALUES
			(93,'AF','Afghanistan'),
			(358,'AX','Aland Islands'),
			(355,'AL','Albania'),
			(213,'DZ','Algeria'),
			(1684,'AS','American Samoa'),
			(376,'AD','Andorra'),
			(244,'AO','Angola'),
			(1264,'AI','Anguilla'),
			(672,'AQ','Antarctica'),
			(1268,'AG','Antigua and Barbuda'),
			(54,'AR','Argentina'),
			(374,'AM','Armenia'),
			(297,'AW','Aruba'),
			(61,'AU','Australia'),
			(43,'AT','Austria'),
			(994,'AZ','Azerbaijan'),
			(1242,'BS','Bahamas'),
			(973,'BH','Bahrain'),
			(880,'BD','Bangladesh'),
			(1246,'BB','Barbados'),
			(375,'BY','Belarus'),
			(32,'BE','Belgium'),
			(501,'BZ','Belize'),
			(229,'BJ','Benin'),
			(1441,'BM','Bermuda'),
			(975,'BT','Bhutan'),
			(591,'BO','Bolivia'),
			(599,'BQ','Bonaire, Sint Eustatius and Saba'),
			(387,'BA','Bosnia and Herzegovina'),
			(267,'BW','Botswana'),
			(55,'BV','Bouvet Island'),
			(55,'BR','Brazil'),
			(246,'IO','British Indian Ocean Territory'),
			(673,'BN','Brunei Darussalam'),
			(359,'BG','Bulgaria'),
			(226,'BF','Burkina Faso'),
			(257,'BI','Burundi'),
			(855,'KH','Cambodia'),
			(237,'CM','Cameroon'),
			(1,'CA','Canada'),
			(238,'CV','Cape Verde'),
			(1345,'KY','Cayman Islands'),
			(236,'CF','Central African Republic'),
			(235,'TD','Chad'),
			(56,'CL','Chile'),
			(86,'CN','China'),
			(61,'CX','Christmas Island'),
			(672,'CC','Cocos (Keeling) Islands'),
			(57,'CO','Colombia'),
			(269,'KM','Comoros'),
			(242,'CG','Congo'),
			(242,'CD','Congo, Democratic Republic of the Congo'),
			(682,'CK','Cook Islands'),
			(506,'CR','Costa Rica'),
			(225,'CI','Cote D\'Ivoire'),
			(385,'HR','Croatia'),
			(53,'CU','Cuba'),
			(599,'CW','Curacao'),
			(357,'CY','Cyprus'),
			(420,'CZ','Czech Republic'),
			(45,'DK','Denmark'),
			(253,'DJ','Djibouti'),
			(1767,'DM','Dominica'),
			(1809,'DO','Dominican Republic'),
			(593,'EC','Ecuador'),
			(20,'EG','Egypt'),
			(503,'SV','El Salvador'),
			(240,'GQ','Equatorial Guinea'),
			(291,'ER','Eritrea'),
			(372,'EE','Estonia'),
			(251,'ET','Ethiopia'),
			(500,'FK','Falkland Islands (Malvinas)'),
			(298,'FO','Faroe Islands'),
			(679,'FJ','Fiji'),
			(358,'FI','Finland'),
			(33,'FR','France'),
			(594,'GF','French Guiana'),
			(689,'PF','French Polynesia'),
			(262,'TF','French Southern Territories'),
			(241,'GA','Gabon'),
			(220,'GM','Gambia'),
			(995,'GE','Georgia'),
			(49,'DE','Germany'),
			(233,'GH','Ghana'),
			(350,'GI','Gibraltar'),
			(30,'GR','Greece'),
			(299,'GL','Greenland'),
			(1473,'GD','Grenada'),
			(590,'GP','Guadeloupe'),
			(1671,'GU','Guam'),
			(502,'GT','Guatemala'),
			(44,'GG','Guernsey'),
			(224,'GN','Guinea'),
			(245,'GW','Guinea-Bissau'),
			(592,'GY','Guyana'),
			(509,'HT','Haiti'),
			(0,'HM','Heard Island and Mcdonald Islands'),
			(39,'VA','Holy See (Vatican City State)'),
			(504,'HN','Honduras'),
			(852,'HK','Hong Kong'),
			(36,'HU','Hungary'),
			(354,'IS','Iceland'),
			(91,'IN','India'),
			(62,'ID','Indonesia'),
			(98,'IR','Iran, Islamic Republic of'),
			(964,'IQ','Iraq'),
			(353,'IE','Ireland'),
			(44,'IM','Isle of Man'),
			(972,'IL','Israel'),
			(39,'IT','Italy'),
			(1876,'JM','Jamaica'),
			(81,'JP','Japan'),
			(44,'JE','Jersey'),
			(962,'JO','Jordan'),
			(7,'KZ','Kazakhstan'),
			(254,'KE','Kenya'),
			(686,'KI','Kiribati'),
			(850,'KP','Korea, Democratic People\'s Republic of'),
			(82,'KR','Korea, Republic of'),
			(381,'XK','Kosovo'),
			(965,'KW','Kuwait'),
			(996,'KG','Kyrgyzstan'),
			(856,'LA','Lao People\'s Democratic Republic'),
			(371,'LV','Latvia'),
			(961,'LB','Lebanon'),
			(266,'LS','Lesotho'),
			(231,'LR','Liberia'),
			(218,'LY','Libyan Arab Jamahiriya'),
			(423,'LI','Liechtenstein'),
			(370,'LT','Lithuania'),
			(352,'LU','Luxembourg'),
			(853,'MO','Macao'),
			(389,'MK','Macedonia, the Former Yugoslav Republic of'),
			(261,'MG','Madagascar'),
			(265,'MW','Malawi'),
			(60,'MY','Malaysia'),
			(960,'MV','Maldives'),
			(223,'ML','Mali'),
			(356,'MT','Malta'),
			(692,'MH','Marshall Islands'),
			(596,'MQ','Martinique'),
			(222,'MR','Mauritania'),
			(230,'MU','Mauritius'),
			(269,'YT','Mayotte'),
			(52,'MX','Mexico'),
			(691,'FM','Micronesia, Federated States of'),
			(373,'MD','Moldova, Republic of'),
			(377,'MC','Monaco'),
			(976,'MN','Mongolia'),
			(382,'ME','Montenegro'),
			(1664,'MS','Montserrat'),
			(212,'MA','Morocco'),
			(258,'MZ','Mozambique'),
			(95,'MM','Myanmar'),
			(264,'NA','Namibia'),
			(674,'NR','Nauru'),
			(977,'NP','Nepal'),
			(31,'NL','Netherlands'),
			(599,'AN','Netherlands Antilles'),
			(687,'NC','New Caledonia'),
			(64,'NZ','New Zealand'),
			(505,'NI','Nicaragua'),
			(227,'NE','Niger'),
			(234,'NG','Nigeria'),
			(683,'NU','Niue'),
			(672,'NF','Norfolk Island'),
			(1670,'MP','Northern Mariana Islands'),
			(47,'NO','Norway'),
			(968,'OM','Oman'),
			(92,'PK','Pakistan'),
			(680,'PW','Palau'),
			(970,'PS','Palestinian Territory, Occupied'),
			(507,'PA','Panama'),
			(675,'PG','Papua New Guinea'),
			(595,'PY','Paraguay'),
			(51,'PE','Peru'),
			(63,'PH','Philippines'),
			(64,'PN','Pitcairn'),
			(48,'PL','Poland'),
			(351,'PT','Portugal'),
			(1787,'PR','Puerto Rico'),
			(974,'QA','Qatar'),
			(262,'RE','Reunion'),
			(40,'RO','Romania'),
			(70,'RU','Russian Federation'),
			(250,'RW','Rwanda'),
			(590,'BL','Saint Barthelemy'),
			(290,'SH','Saint Helena'),
			(1869,'KN','Saint Kitts and Nevis'),
			(1758,'LC','Saint Lucia'),
			(590,'MF','Saint Martin'),
			(508,'PM','Saint Pierre and Miquelon'),
			(1784,'VC','Saint Vincent and the Grenadines'),
			(684,'WS','Samoa'),
			(378,'SM','San Marino'),
			(239,'ST','Sao Tome and Principe'),
			(966,'SA','Saudi Arabia'),
			(221,'SN','Senegal'),
			(381,'RS','Serbia'),
			(381,'CS','Serbia and Montenegro'),
			(248,'SC','Seychelles'),
			(232,'SL','Sierra Leone'),
			(65,'SG','Singapore'),
			(1,'SX','Sint Maarten'),
			(421,'SK','Slovakia'),
			(386,'SI','Slovenia'),
			(677,'SB','Solomon Islands'),
			(252,'SO','Somalia'),
			(27,'ZA','South Africa'),
			(500,'GS','South Georgia and the South Sandwich Islands'),
			(211,'SS','South Sudan'),
			(34,'ES','Spain'),
			(94,'LK','Sri Lanka'),
			(249,'SD','Sudan'),
			(597,'SR','Suriname'),
			(47,'SJ','Svalbard and Jan Mayen'),
			(268,'SZ','Swaziland'),
			(46,'SE','Sweden'),
			(41,'CH','Switzerland'),
			(963,'SY','Syrian Arab Republic'),
			(886,'TW','Taiwan, Province of China'),
			(992,'TJ','Tajikistan'),
			(255,'TZ','Tanzania, United Republic of'),
			(66,'TH','Thailand'),
			(670,'TL','Timor-Leste'),
			(228,'TG','Togo'),
			(690,'TK','Tokelau'),
			(676,'TO','Tonga'),
			(1868,'TT','Trinidad and Tobago'),
			(216,'TN','Tunisia'),
			(90,'TR','Turkey'),
			(7370,'TM','Turkmenistan'),
			(1649,'TC','Turks and Caicos Islands'),
			(688,'TV','Tuvalu'),
			(256,'UG','Uganda'),
			(380,'UA','Ukraine'),
			(971,'AE','United Arab Emirates'),
			(44,'GB','United Kingdom'),
			(1,'US','United States'),
			(1,'UM','United States Minor Outlying Islands'),
			(598,'UY','Uruguay'),
			(998,'UZ','Uzbekistan'),
			(678,'VU','Vanuatu'),
			(58,'VE','Venezuela'),
			(84,'VN','Viet Nam'),
			(1284,'VG','Virgin Islands, British'),
			(1340,'VI','Virgin Islands, U.s.'),
			(681,'WF','Wallis and Futuna'),
			(212,'EH','Western Sahara'),
			(967,'YE','Yemen'),
			(260,'ZM','Zambia'),
			(263,'ZW','Zimbabwe');
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
			CREATE TABLE IF NOT EXISTS user_contact_number (
				user_id BINARY(16),
				country_phone_code VARCHAR(5) NOT NULL,
				country_code VARCHAR(2) NOT NULL,
				number VARCHAR(15),
				PRIMARY KEY(country_code, number),
				FOREIGN KEY(user_id) REFERENCES user(id),
				FOREIGN KEY(country_phone_code, country_code) REFERENCES phone_number_country_code(phone_code, country_code),
				CONSTRAINT phone_number_check CHECK(LENGTH(number) >= 7 AND LENGTH(number) <= 15)
			);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// add user id as a foreign key later
	query!(
		r#"
		ALTER TABLE user
		ADD CONSTRAINT 
		FOREIGN KEY(id, backup_email_local, backup_email_domain_id) REFERENCES personal_email(user_id, email_local, domain_id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// add user id as a foreign key
	query!(
		r#"
		ALTER TABLE user
		ADD CONSTRAINT 
		FOREIGN KEY ( 
			backup_phone_number,
			backup_country_code
		) 
		REFERENCES user_contact_number ( 
			number,
			country_code
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_unverified_email_address (
			/* Personal email address */
			email_address VARCHAR(320),
			user_id BINARY(16) NOT NULL,
			verification_token_hash TEXT NOT NULL,
			verification_token_expiry BIGINT UNSIGNED NOT NULL,
			
			PRIMARY KEY(email_address, user_id),
			FOREIGN KEY(user_id) REFERENCES user(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_to_sign_up (
			username VARCHAR(100) PRIMARY KEY,
			account_type ENUM('personal', 'organisation') NOT NULL,
			
			/* Personal email address OR backup email */
			email_address VARCHAR(320) NOT NULL,

			/* Organisation email address */
			email_local VARCHAR(160),
			domain_name VARCHAR(100),

			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,

			organisation_name VARCHAR(100),

			otp_hash TEXT NOT NULL,
			otp_expiry BIGINT UNSIGNED NOT NULL,

			CONSTRAINT CHECK
			(
				(
					account_type = 'personal' AND
					(
						email_local IS NULL AND
						domain_name IS NULL AND
						organisation_name IS NULL
					)
				) OR
				(
					account_type = 'organisation' AND
					(
						email_local IS NOT NULL AND
						domain_name IS NOT NULL AND
						organisation_name IS NOT NULL
					)
				)
			)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn get_user_by_username_or_email(
	connection: &mut Transaction<'_, MySql>,
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
			generic_domain
		ON
			personal_email.domain_id = generic_domain.id
		WHERE
			user.username = ? OR
			CONCAT(personal_email.email_local,'@',generic_domain.name) = ?;
		"#,
		user_id,
		user_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn generate_new_user_id(
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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

// This only retreives user using personal email address/backup email address
// getting a compile time error here
pub async fn get_user_by_email(
	connection: &mut Transaction<'_, MySql>,
	email_local: &str,
	email_domain: &str,
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
		RIGHT JOIN
			generic_domain
		ON
			personal_email.domain_id = generic_domain.id
		WHERE
			personal_email.email_local = ?
		AND
			generic_domain.name = ?
		"#,
		email_local,
		email_domain
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_user_by_username(
	connection: &mut Transaction<'_, MySql>,
	username: &str,
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
		RIGHT JOIN
			generic_domain
		ON
			personal_email.domain_id = generic_domain.id
		WHERE
			username = ?
		"#,
		username
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn get_user_by_user_id(
	connection: &mut Transaction<'_, MySql>,
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
			id = ?
		"#,
		user_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn set_user_to_be_signed_up(
	connection: &mut Transaction<'_, MySql>,
	email: UserEmailAddressSignUp,
	username: &str,
	password: &str,
	(first_name, last_name): (&str, &str),
	otp_hash: &str,
	otp_expiry: u64,
) -> Result<(), sqlx::Error> {
	match email {
		UserEmailAddressSignUp::Personal(email) => {
			query!(
				r#"
				INSERT INTO
					user_to_sign_up
				VALUES
					(?, 'personal', ?, NULL, NULL, ?, ?, ?, NULL, ?, ?)
				ON DUPLICATE KEY UPDATE
					account_type = 'personal',
					
					email_address = ?,
					
					email_local = NULL,
					domain_name = NULL,
					
					organisation_name = NULL,
					
					password = ?,
					first_name = ?,
					last_name = ?,
					otp_hash = ?,
					otp_expiry = ?;
				"#,
				username,
				email,
				password,
				first_name,
				last_name,
				otp_hash,
				otp_expiry,
				email,
				password,
				first_name,
				last_name,
				otp_hash,
				otp_expiry
			)
			.execute(connection)
			.await?;
		}
		UserEmailAddressSignUp::Organisation {
			email_local,
			domain_name,
			organisation_name,
			backup_email,
		} => {
			query!(
				r#"
				INSERT INTO
					user_to_sign_up
				VALUES
					(?, 'organisation', ?, ?, ?, ?, ?, ?, ?, ?, ?)
				ON DUPLICATE KEY UPDATE
					account_type = 'organisation',
					
					email_address = ?,
					
					email_local = ?,
					domain_name = ?,
					
					password = ?,
					first_name = ?,
					last_name = ?,
					
					organisation_name = ?,
					
					otp_hash = ?,
					otp_expiry = ?;
				"#,
				username,
				backup_email,
				email_local,
				domain_name,
				password,
				first_name,
				last_name,
				organisation_name,
				otp_hash,
				otp_expiry,
				backup_email,
				email_local,
				domain_name,
				password,
				first_name,
				last_name,
				organisation_name,
				otp_hash,
				otp_expiry
			)
			.execute(connection)
			.await?;
		}
	}

	Ok(())
}

pub async fn get_user_to_sign_up_by_username(
	connection: &mut Transaction<'_, MySql>,
	username: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			username = ?
		"#,
		username
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(UserToSignUp {
		username: row.username,
		email: if row.account_type == "personal" {
			UserEmailAddressSignUp::Personal(row.email_address.clone())
		} else if row.account_type == "organisation" {
			UserEmailAddressSignUp::Organisation {
				email_local: row.email_local.unwrap(),
				domain_name: row.domain_name.unwrap(),
				organisation_name: row.organisation_name.unwrap(),
				backup_email: row.email_address.clone(),
			}
		} else {
			panic!("Unknown account_type");
		},
		backup_email: row.email_address,
		password: row.password,
		otp_hash: row.otp_hash,
		otp_expiry: row.otp_expiry,
		first_name: row.first_name,
		last_name: row.last_name,
	}))
}

pub async fn get_user_to_sign_up_by_organisation_name(
	connection: &mut Transaction<'_, MySql>,
	organisation_name: &str,
) -> Result<Option<UserToSignUp>, sqlx::Error> {
	let now = utils::get_current_time();
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			organisation_name = ? AND
			otp_expiry < ?
		"#,
		organisation_name,
		now,
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	Ok(Some(UserToSignUp {
		username: row.username,
		email: if row.account_type == "organisation" {
			UserEmailAddressSignUp::Organisation {
				email_local: row.email_local.unwrap(),
				domain_name: row.domain_name.unwrap(),
				organisation_name: row.organisation_name.unwrap(),
				backup_email: row.email_address.clone(),
			}
		} else {
			panic!("account_type wasn't organisation for an organisation");
		},
		backup_email: row.email_address,
		password: row.password,
		otp_hash: row.otp_hash,
		otp_expiry: row.otp_expiry,
		first_name: row.first_name,
		last_name: row.last_name,
	}))
}
pub async fn add_to_personal_email(
	connection: &mut Transaction<'_, MySql>,
	user_id: Vec<u8>,
	email_local: &str,
	domain_id: Vec<u8>,
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

pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut Transaction<'_, MySql>,
	email: &str,
	user_id: &[u8],
	verification_token: &str,
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_unverified_email_address
		VALUES
			(?, ?, ?, ?)
		ON DUPLICATE KEY UPDATE
			verification_token_hash = ?,
			verification_token_expiry = ?;
		"#,
		email,
		user_id,
		verification_token,
		token_expiry,
		verification_token,
		token_expiry
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_personal_email_to_be_verified_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	email: &str,
) -> Result<Option<PersonalEmailToBeVerified>, sqlx::Error> {
	let rows = query_as!(
		PersonalEmailToBeVerified,
		r#"
		SELECT
			*
		FROM
			user_unverified_email_address
		WHERE
			user_id = ? AND
			email_address = ?;
		"#,
		user_id,
		email
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn add_user_id_to_email(
	connection: &mut Transaction<'_, MySql>,
	user_id: Vec<u8>,
	email_domain_id: Vec<u8>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			UPDATE  
				personal_email
			SET
				user_id = ?
			WHERE
				domain_id = ?
		"#,
		user_id,
		email_domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_email_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: Option<&[u8]>,
	email: UserEmailAddress,
	email_type: AccountType,
) -> Result<(), sqlx::Error> {
	match email_type {
		AccountType::Personal => {
			query!(
				r#"
				INSERT INTO
					personal_email
				VALUES
					(?, ?, ?);
				"#,
				user_id,
				email.email_local,
				email.domain_id
			)
			.execute(connection)
			.await?;
		}
		AccountType::Organisation => {
			query!(
				r#"
				INSERT INTO
					organisation_email
				VALUES
					(?, ?, ?);
				"#,
				user_id,
				email.email_local,
				email.domain_id
			)
			.execute(connection)
			.await?;
		}
	}

	Ok(())
}

pub async fn delete_user_to_be_signed_up(
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	username: &str,
	password: &str,
	backup_email_local: &str,
	backup_email_domain_id: Vec<u8>,
	backup_phone_number_country_code: Option<&str>,
	backup_country_code: Option<&str>,
	backup_phone_number: Option<&str>,
	(first_name, last_name): (&str, &str),
	created: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user
		VALUES
			(?, ?, ?, ?, ?,NULL, NULL, NULL, ?, ?, ?, ?, ?, ?);
		"#,
		user_id,
		username,
		password,
		first_name,
		last_name,
		created,
		backup_email_local,
		backup_email_domain_id,
		backup_phone_number_country_code,
		backup_country_code,
		backup_phone_number,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_user_login(
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
	login_id: &[u8],
) -> Result<Option<UserLogin>, sqlx::Error> {
	let rows = query_as!(
		UserLogin,
		r#"
		SELECT * FROM
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
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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

pub async fn set_refresh_token_expiry(
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
) -> Result<Vec<Organisation>, sqlx::Error> {
	let organisations = query_as!(
		Organisation,
		r#"
		SELECT DISTINCT
			organisation.id,
			organisation.name,
			organisation.super_admin_id,
			organisation.active as `active!: bool`,
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

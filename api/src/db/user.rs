use sqlx::{MySql, Transaction};
use uuid::Uuid;

use crate::{
	models::db_mapping::{
		Organisation,
		PasswordResetRequest,
		PersonalDomain,
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
			user_id BINARY(16) NOT NULL,
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
			country_code CHAR(2) PRIMARY KEY,
			phone_code VARCHAR(5) NOT NULL,
			country_name VARCHAR(80) NOT NULL
		);	
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS user_contact_number (
			user_id BINARY(16),
			country_code VARCHAR(2) NOT NULL,
			number VARCHAR(15),
			PRIMARY KEY(country_code, number),
			FOREIGN KEY(user_id) REFERENCES user(id),
			FOREIGN KEY(country_code) REFERENCES phone_number_country_code(country_code),
			CONSTRAINT CHECK(LENGTH(number) >= 7 AND LENGTH(number) <= 15)
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
		FOREIGN KEY (
			backup_email_local, 
			backup_email_domain_id
		) 
		REFERENCES personal_email (
			email_local, 
			domain_id
		);
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
			email_local VARCHAR(64),
			email_domain_id BINARY(16) NOT NULL,
			user_id BINARY(16) NOT NULL,
			verification_token_hash TEXT NOT NULL,
			verification_token_expiry BIGINT UNSIGNED NOT NULL,
			
			PRIMARY KEY(email_local, email_domain_id, user_id),
			FOREIGN KEY(user_id) REFERENCES user(id),
			FOREIGN KEY(email_domain_id) REFERENCES personal_domain(id)
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
			personal_email_local VARCHAR(64) NOT NULL,
			personal_email_domain_id BINARY(16) NOT NULL,

			/* Organisation email address */
			org_email_local VARCHAR(160),
			org_domain_name VARCHAR(100),

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
			FOREIGN KEY(personal_email_domain_id) REFERENCES personal_domain(id)
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
			('AF', '93', 'Afghanistan'),
			('AX', '358', 'Aland Islands'),
			('AL', '355', 'Albania'),
			('DZ', '213', 'Algeria'),
			('AS', '1684', 'American Samoa'),
			('AD', '376', 'Andorra'),
			('AO', '244', 'Angola'),
			('AI', '1264', 'Anguilla'),
			('AQ', '672', 'Antarctica'),
			('AG', '1268', 'Antigua and Barbuda'),
			('AR', '54', 'Argentina'),
			('AM', '374', 'Armenia'),
			('AW', '297', 'Aruba'),
			('AU', '61', 'Australia'),
			('AT', '43', 'Austria'),
			('AZ', '994', 'Azerbaijan'),
			('BS', '1242', 'Bahamas'),
			('BH', '973', 'Bahrain'),
			('BD', '880', 'Bangladesh'),
			('BB', '1246', 'Barbados'),
			('BY', '375', 'Belarus'),
			('BE', '32', 'Belgium'),
			('BZ', '501', 'Belize'),
			('BJ', '229', 'Benin'),
			('BM', '1441', 'Bermuda'),
			('BT', '975', 'Bhutan'),
			('BO', '591', 'Bolivia'),
			('BQ', '599', 'Bonaire, Sint Eustatius and Saba'),
			('BA', '387', 'Bosnia and Herzegovina'),
			('BW', '267', 'Botswana'),
			('BV', '55', 'Bouvet Island'),
			('BR', '55', 'Brazil'),
			('IO', '246', 'British Indian Ocean Territory'),
			('BN', '673', 'Brunei Darussalam'),
			('BG', '359', 'Bulgaria'),
			('BF', '226', 'Burkina Faso'),
			('BI', '257', 'Burundi'),
			('KH', '855', 'Cambodia'),
			('CM', '237', 'Cameroon'),
			('CA', '1', 'Canada'),
			('CV', '238', 'Cape Verde'),
			('KY', '1345', 'Cayman Islands'),
			('CF', '236', 'Central African Republic'),
			('TD', '235', 'Chad'),
			('CL', '56', 'Chile'),
			('CN', '86', 'China'),
			('CX', '61', 'Christmas Island'),
			('CC', '672', 'Cocos (Keeling) Islands'),
			('CO', '57', 'Colombia'),
			('KM', '269', 'Comoros'),
			('CG', '242', 'Congo'),
			('CD', '242', 'Congo, Democratic Republic of the Congo'),
			('CK', '682', 'Cook Islands'),
			('CR', '506', 'Costa Rica'),
			('CI', '225', 'Cote D\'Ivoire'),
			('HR', '385', 'Croatia'),
			('CU', '53', 'Cuba'),
			('CW', '599', 'Curacao'),
			('CY', '357', 'Cyprus'),
			('CZ', '420', 'Czech Republic'),
			('DK', '45', 'Denmark'),
			('DJ', '253', 'Djibouti'),
			('DM', '1767', 'Dominica'),
			('DO', '1809', 'Dominican Republic'),
			('EC', '593', 'Ecuador'),
			('EG', '20', 'Egypt'),
			('SV', '503', 'El Salvador'),
			('GQ', '240', 'Equatorial Guinea'),
			('ER', '291', 'Eritrea'),
			('EE', '372', 'Estonia'),
			('ET', '251', 'Ethiopia'),
			('FK', '500', 'Falkland Islands (Malvinas)'),
			('FO', '298', 'Faroe Islands'),
			('FJ', '679', 'Fiji'),
			('FI', '358', 'Finland'),
			('FR', '33', 'France'),
			('GF', '594', 'French Guiana'),
			('PF', '689', 'French Polynesia'),
			('TF', '262', 'French Southern Territories'),
			('GA', '241', 'Gabon'),
			('GM', '220', 'Gambia'),
			('GE', '995', 'Georgia'),
			('DE', '49', 'Germany'),
			('GH', '233', 'Ghana'),
			('GI', '350', 'Gibraltar'),
			('GR', '30', 'Greece'),
			('GL', '299', 'Greenland'),
			('GD', '1473', 'Grenada'),
			('GP', '590', 'Guadeloupe'),
			('GU', '1671', 'Guam'),
			('GT', '502', 'Guatemala'),
			('GG', '44', 'Guernsey'),
			('GN', '224', 'Guinea'),
			('GW', '245', 'Guinea-Bissau'),
			('GY', '592', 'Guyana'),
			('HT', '509', 'Haiti'),
			('HM', '0', 'Heard Island and Mcdonald Islands'),
			('VA', '39', 'Holy See (Vatican City State)'),
			('HN', '504', 'Honduras'),
			('HK', '852', 'Hong Kong'),
			('HU', '36', 'Hungary'),
			('IS', '354', 'Iceland'),
			('IN', '91', 'India'),
			('ID', '62', 'Indonesia'),
			('IR', '98', 'Iran, Islamic Republic of'),
			('IQ', '964', 'Iraq'),
			('IE', '353', 'Ireland'),
			('IM', '44', 'Isle of Man'),
			('IL', '972', 'Israel'),
			('IT', '39', 'Italy'),
			('JM', '1876', 'Jamaica'),
			('JP', '81', 'Japan'),
			('JE', '44', 'Jersey'),
			('JO', '962', 'Jordan'),
			('KZ', '7', 'Kazakhstan'),
			('KE', '254', 'Kenya'),
			('KI', '686', 'Kiribati'),
			('KP', '850', 'Korea, Democratic People\'s Republic of'),
			('KR', '82', 'Korea, Republic of'),
			('XK', '381', 'Kosovo'),
			('KW', '965', 'Kuwait'),
			('KG', '996', 'Kyrgyzstan'),
			('LA', '856', 'Lao People\'s Democratic Republic'),
			('LV', '371', 'Latvia'),
			('LB', '961', 'Lebanon'),
			('LS', '266', 'Lesotho'),
			('LR', '231', 'Liberia'),
			('LY', '218', 'Libyan Arab Jamahiriya'),
			('LI', '423', 'Liechtenstein'),
			('LT', '370', 'Lithuania'),
			('LU', '352', 'Luxembourg'),
			('MO', '853', 'Macao'),
			('MK', '389', 'Macedonia, the Former Yugoslav Republic of'),
			('MG', '261', 'Madagascar'),
			('MW', '265', 'Malawi'),
			('MY', '60', 'Malaysia'),
			('MV', '960', 'Maldives'),
			('ML', '223', 'Mali'),
			('MT', '356', 'Malta'),
			('MH', '692', 'Marshall Islands'),
			('MQ', '596', 'Martinique'),
			('MR', '222', 'Mauritania'),
			('MU', '230', 'Mauritius'),
			('YT', '269', 'Mayotte'),
			('MX', '52', 'Mexico'),
			('FM', '691', 'Micronesia, Federated States of'),
			('MD', '373', 'Moldova, Republic of'),
			('MC', '377', 'Monaco'),
			('MN', '976', 'Mongolia'),
			('ME', '382', 'Montenegro'),
			('MS', '1664', 'Montserrat'),
			('MA', '212', 'Morocco'),
			('MZ', '258', 'Mozambique'),
			('MM', '95', 'Myanmar'),
			('NA', '264', 'Namibia'),
			('NR', '674', 'Nauru'),
			('NP', '977', 'Nepal'),
			('NL', '31', 'Netherlands'),
			('AN', '599', 'Netherlands Antilles'),
			('NC', '687', 'New Caledonia'),
			('NZ', '64', 'New Zealand'),
			('NI', '505', 'Nicaragua'),
			('NE', '227', 'Niger'),
			('NG', '234', 'Nigeria'),
			('NU', '683', 'Niue'),
			('NF', '672', 'Norfolk Island'),
			('MP', '1670', 'Northern Mariana Islands'),
			('NO', '47', 'Norway'),
			('OM', '968', 'Oman'),
			('PK', '92', 'Pakistan'),
			('PW', '680', 'Palau'),
			('PS', '970', 'Palestinian Territory, Occupied'),
			('PA', '507', 'Panama'),
			('PG', '675', 'Papua New Guinea'),
			('PY', '595', 'Paraguay'),
			('PE', '51', 'Peru'),
			('PH', '63', 'Philippines'),
			('PN', '64', 'Pitcairn'),
			('PL', '48', 'Poland'),
			('PT', '351', 'Portugal'),
			('PR', '1787', 'Puerto Rico'),
			('QA', '974', 'Qatar'),
			('RE', '262', 'Reunion'),
			('RO', '40', 'Romania'),
			('RU', '70', 'Russian Federation'),
			('RW', '250', 'Rwanda'),
			('BL', '590', 'Saint Barthelemy'),
			('SH', '290', 'Saint Helena'),
			('KN', '1869', 'Saint Kitts and Nevis'),
			('LC', '1758', 'Saint Lucia'),
			('MF', '590', 'Saint Martin'),
			('PM', '508', 'Saint Pierre and Miquelon'),
			('VC', '1784', 'Saint Vincent and the Grenadines'),
			('WS', '684', 'Samoa'),
			('SM', '378', 'San Marino'),
			('ST', '239', 'Sao Tome and Principe'),
			('SA', '966', 'Saudi Arabia'),
			('SN', '221', 'Senegal'),
			('RS', '381', 'Serbia'),
			('CS', '381', 'Serbia and Montenegro'),
			('SC', '248', 'Seychelles'),
			('SL', '232', 'Sierra Leone'),
			('SG', '65', 'Singapore'),
			('SX', '1', 'Sint Maarten'),
			('SK', '421', 'Slovakia'),
			('SI', '386', 'Slovenia'),
			('SB', '677', 'Solomon Islands'),
			('SO', '252', 'Somalia'),
			('ZA', '27', 'South Africa'),
			('GS', '500', 'South Georgia and the South Sandwich Islands'),
			('SS', '211', 'South Sudan'),
			('ES', '34', 'Spain'),
			('LK', '94', 'Sri Lanka'),
			('SD', '249', 'Sudan'),
			('SR', '597', 'Suriname'),
			('SJ', '47', 'Svalbard and Jan Mayen'),
			('SZ', '268', 'Swaziland'),
			('SE', '46', 'Sweden'),
			('CH', '41', 'Switzerland'),
			('SY', '963', 'Syrian Arab Republic'),
			('TW', '886', 'Taiwan, Province of China'),
			('TJ', '992', 'Tajikistan'),
			('TZ', '255', 'Tanzania, United Republic of'),
			('TH', '66', 'Thailand'),
			('TL', '670', 'Timor-Leste'),
			('TG', '228', 'Togo'),
			('TK', '690', 'Tokelau'),
			('TO', '676', 'Tonga'),
			('TT', '1868', 'Trinidad and Tobago'),
			('TN', '216', 'Tunisia'),
			('TR', '90', 'Turkey'),
			('TM', '7370', 'Turkmenistan'),
			('TC', '1649', 'Turks and Caicos Islands'),
			('TV', '688', 'Tuvalu'),
			('UG', '256', 'Uganda'),
			('UA', '380', 'Ukraine'),
			('AE', '971', 'United Arab Emirates'),
			('GB', '44', 'United Kingdom'),
			('US', '1', 'United States'),
			('UM', '1', 'United States Minor Outlying Islands'),
			('UY', '598', 'Uruguay'),
			('UZ', '998', 'Uzbekistan'),
			('VU', '678', 'Vanuatu'),
			('VE', '58', 'Venezuela'),
			('VN', '84', 'Viet Nam'),
			('VG', '1284', 'Virgin Islands, British'),
			('VI', '1340', 'Virgin Islands, U.s.'),
			('WF', '681', 'Wallis and Futuna'),
			('EH', '212', 'Western Sahara'),
			('YE', '967', 'Yemen'),
			('ZM', '260', 'Zambia'),
			('ZW', '263', 'Zimbabwe');
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
			organisation_email
		ON
			organisation_email.user_id = user.id
		LEFT JOIN
			generic_domain
		ON
			personal_email.domain_id = generic_domain.id OR
			organisation_email.domain_id = generic_domain.id
		WHERE
			user.username = ? OR
			CONCAT(personal_email.email_local, '@', generic_domain.name) = ? OR
			CONCAT(organisation_email.email_local, '@', generic_domain.name) = ?;
		"#,
		user_id,
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
pub async fn get_user_by_email(
	connection: &mut Transaction<'_, MySql>,
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
			generic_domain
		ON
			personal_email.domain_id = generic_domain.id OR
			organisation_email.domain_id = generic_domain.id
		WHERE
			CONCAT(personal_email.email_local, '@', generic_domain.name) = ? OR
			CONCAT(organisation_email.email_local, '@', generic_domain.name) = ?;
		"#,
		email,
		email
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

pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut Transaction<'_, MySql>,
	email_local: &str,
	domain_id: &[u8],
	user_id: &[u8],
	verification_token: &str,
	token_expiry: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_unverified_email_address
		VALUES
			(?, ?, ?, ?, ?)
		ON DUPLICATE KEY UPDATE
			verification_token_hash = ?,
			verification_token_expiry = ?;
		"#,
		email_local,
		domain_id,
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
			user_unverified_email_address.*
		FROM
			user_unverified_email_address
		INNER JOIN
			generic_domain
		ON
			user_unverified_email_address.email_domain_id = generic_domain.id
		WHERE
			user_id = ? AND
			CONCAT(email_local, '@', generic_domain.name) = ?;
		"#,
		user_id,
		email
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn set_user_id_for_email(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	email_domain_id: &[u8],
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

pub async fn add_orphaned_personal_email_for_user(
	connection: &mut Transaction<'_, MySql>,
	email: &UserEmailAddress,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			personal_email
		VALUES
			(NULL, ?, ?);
		"#,
		email.email_local,
		email.domain_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn add_email_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
	email: &UserEmailAddress,
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
	backup_email_domain_id: &[u8],
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
			(?, ?, ?, ?, ?, NULL, NULL, NULL, ?, ?, ?, ?, ?);
		"#,
		user_id,
		username,
		password,
		first_name,
		last_name,
		created,
		backup_email_local,
		backup_email_domain_id,
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

pub async fn get_backup_email_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
) -> Result<Option<(String, PersonalDomain)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			user.backup_email_local,
			user.backup_email_domain_id,
			generic_domain.name
		FROM
			user
		INNER JOIN
			personal_domain
		ON
			personal_domain.id = user.backup_email_domain_id
		INNER JOIN
			generic_domain
		ON
			personal_domain.id = generic_domain.id
		WHERE
			user.id = ?;
		"#,
		user_id
	)
	.fetch_all(connection)
	.await?;

	if rows.is_empty() {
		return Ok(None);
	}
	let row = rows.into_iter().next().unwrap();

	if row.backup_email_local.is_none() {
		return Ok(None);
	}

	Ok(Some((
		row.backup_email_local.unwrap(),
		PersonalDomain {
			id: row.backup_email_domain_id.unwrap(),
			name: row.name,
		},
	)))
}

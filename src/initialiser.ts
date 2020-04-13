import { pool, getDatabaseMajorVersion, getDatabaseMinorVersion, getDatabaseIncrementVersion } from './models/database';
import { setDatabaseVersion } from './models/database';
import { mysql } from './config/config';

export const DatabaseVersion = {
	major: 1,
	minor: 0,
	increment: 2
};

export async function initialize() {
	console.log('Initialising database');
	const rows = await pool.query('SHOW TABLES;');

	// If no tables exist in the database, initialise fresh
	if (rows.length === 0) {
		console.log('No tables exist. Creating fresh');
		// Create all tables
		await initialiseMeta();
		await initializeFiles();
		await initializeUserAccounts();
		await initializeGroups();

		// Set the database schema version
		await setDatabaseVersion(DatabaseVersion.major, DatabaseVersion.minor, DatabaseVersion.increment);
		console.log('Database created fresh');
		return;
	}

	console.log('Tables already exist. Performing a migration');
	// If it already exists, perform a migration with the known version
	const majorVersion = await getDatabaseMajorVersion();
	const minorVersion = await getDatabaseMinorVersion();
	const incrementVersion = await getDatabaseIncrementVersion();

	if (majorVersion === DatabaseVersion.major && minorVersion === DatabaseVersion.minor && incrementVersion === DatabaseVersion.increment) {
		console.log('Database already in latest version. Migration not required.');
	} else {
		console.log(`Migrating from ${majorVersion}.${minorVersion}.${incrementVersion}`);
		await migrateDatabase(majorVersion, minorVersion, incrementVersion);
	}
}

// tslint:disable: no-switch-case-fall-through
async function migrateDatabase(majorVersion: number, minorVersion: number, increment: number) {
	// Intentional fall through
	switch (`${majorVersion}.${minorVersion}.${increment}`) {
		case '1.0.0':
			break;
	}
	await setDatabaseVersion(DatabaseVersion.major, DatabaseVersion.minor, DatabaseVersion.increment);
}
// tslint:enable: no-switch-case-fall-through

async function initialiseMeta() {
	await pool.query(
		`
		CREATE TABLE IF NOT EXISTS meta_data (
			metaId VARCHAR(100) PRIMARY KEY,
			value TEXT
		);
		`
	);
}

async function initializeFiles() {
	await pool.query(
		`
		CREATE TABLE IF NOT EXISTS files (
			fileId VARCHAR(36) PRIMARY KEY,
			contentType VARCHAR(100),
			fileName VARCHAR(150),
			created BIGINT,
			hash VARCHAR(128),
			size BIGINT
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE IF NOT EXISTS access_groups (
			groupId VARCHAR(100) PRIMARY KEY
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE IF NOT EXISTS file_acl_groups (
			fileId VARCHAR(36),
			groupId VARCHAR(100),
			permission VARCHAR(20),
			PRIMARY KEY(fileId, groupId),
			FOREIGN KEY(fileId) REFERENCES files(fileId),
			FOREIGN KEY(groupId) REFERENCES access_groups(groupId)
		);
		`
	);
}

async function initializeUserAccounts() {
	await pool.query(
		`
		CREATE TABLE IF NOT EXISTS users (
			userId VARCHAR(36) PRIMARY KEY,
			username VARCHAR(80) UNIQUE NOT NULL,
			email VARCHAR(150) UNIQUE NOT NULL,
			password VARCHAR(64),
			phone INT(10),
			dob BIGINT,
			bio VARCHAR(150),
			firstName VARCHAR(50),
			lastName VARCHAR(50),
			country VARCHAR(50),
			streetAddress1 VARCHAR(150),
			streetAddress2 VARCHAR(150),
			state VARCHAR(50),
			city VARCHAR(50),
			pincode INT(10),
			created BIGINT
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE IF NOT EXISTS logins (
			authToken VARCHAR(36) PRIMARY KEY,
			authExp BIGINT,
			userId VARCHAR(36),
			lastLogin BIGINT,
			lastActivity BIGINT,
			FOREIGN KEY(userId) REFERENCES users(userId)
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE IF NOT EXISTS invites (
			email VARCHAR(150) PRIMARY KEY,
			username VARCHAR(80) UNIQUE NOT NULL,
			password VARCHAR(64),
			token VARCHAR(64),
			tokenExpiry BIGINT
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE IF NOT EXISTS password_reset_requests (
			userId VARCHAR(36) PRIMARY KEY,
			token VARCHAR(64) UNIQUE,
			tokenExpiry BIGINT,
			FOREIGN KEY(userId) REFERENCES users(userId)
		);
		`
	);
}

async function initializeGroups() {
	await pool.query(
		`
		CREATE TABLE groups (
			groupId VARCHAR(100) PRIMARY KEY,
			name VARCHAR(1024) UNIQUE NOT NULL,
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE user_groups (
			userId VARCHAR() NOT NULL,groupId VARCHAR() NOT NULL,
			PRIMARY KEY(userId, groupId),
			FOREIGN KEY(userId) REFERENCES users(userId),
			FOREIGN KEY(groupId) REFERENCES groups(groupId)
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE resources (
			name VARCHAR() NOT NULL,
			type VARCHAR() NOT NULL,
			resourceID VARCHAR() PRIMARY KEY,
			UNIQUE KEY(name, type)
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE permissions (
			name VARCHAR() NOT NULL,
			permissionID varchar() PRIMARY KEY,
			resourceID varchar(),
			FOREGIN KEY(resourceID) REFERENCES Resources(resourceID)
		);
		`
	);

	await pool.query(
		`
		CREATE TABLE group_permissions(
			groupId VARCHAR() NOT NULL,
			permissionId VARCHAR() NOT NULL,
			PRIMARY KEY(groupId, permissionId)
			FOREIGN KEY(groupId) REFERENCES groups(groupId),
			FOREIGN KEY(permissionID) REFERENCES permissions(permissionId)
		);
		`
	);
}

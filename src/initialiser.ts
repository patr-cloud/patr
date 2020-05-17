import pool from './models/database';
import { createGroup } from './models/database-modules/group';

async function createClients() {
	console.log('Creating clients table');
	await pool.query(
		`
	CREATE TABLE clients(
		clientId VARCHAR(32) PRIMARY KEY,
		clientSecret VARCHAR(32) NOT NULL,
		redirectUris JSON NOT NULL,
		responseTypes JSON NOT NULL,
		grantTypes JSON NOT NULL
	);
	`,
	);
}

async function createUsers() {
	console.log('Creating users table');
	await pool.query(
		`
		CREATE TABLE users(
			userId BINARY(16) PRIMARY KEY,
			username VARCHAR(80) UNIQUE NOT NULL,
			password CHAR(60) NOT NULL,
			email VARCHAR(320) UNIQUE NOT NULL
		  );
		`,
	);
}

async function createGroups() {
	console.log('Creating groups table');
	await pool.query(
		`
		CREATE TABLE user_groups(
			groupId BINARY(16) PRIMARY KEY,
			name VARCHAR(80) UNIQUE NOT NULL,
			FOREIGN KEY(groupId) REFERENCES resources(resourceId) ON DELETE CASCADE
		  );
		`,
	);
}

// async function createUserGroups() {
//     console.log("Creating user_groups table");
//     await pool.query(
//         `
//         CREATE TABLE user_groups(
//             userId VARCHAR(36) NOT NULL,
//             GroupId VARCHAR(36) NOT NULL,
//             PRIMARY KEY(userId, groupId),
//             FOREIGN KEY(userId) REFERENCES users(userId) ON DELETE CASCADE,
//             FOREIGN KEY(groupId) REFERENCES groups(groupId) ON DELETE CASCADE
//           );

//         `
//     )
// }

async function createResources() {
	console.log('Creating resources table');
	await pool.query(
		`
		CREATE TABLE resources(
			resourceId BINARY(16) PRIMARY KEY,
			name VARCHAR(80) UNIQUE NOT NULL,
			type VARCHAR(80) NOT NULL
		  );
		  
		`,
	);
}

async function createPermissions() {
	console.log('Creating permissions table');
	await pool.query(
		`
		CREATE TABLE permissions(
			permissionId BINARY(16) PRIMARY KEY,
			name VARCHAR(80) NOT NULL
		  );
		`,
	);
}

async function createRoles() {
	console.log('Creating roles table');
	await pool.query(
		`
		CREATE TABLE roles(
			roleId BINARY(16) PRIMARY KEY,
			name VARCHAR(80) NOT NULL,
			groupId BINARY(16) NULL,
			FOREIGN KEY(groupId) REFERENCES user_groups(groupId) ON DELETE CASCADE
		);
		`,
	);
}

async function createRolePermissions() {
	console.log('Creating role_permissions Table');
	await pool.query(
		`
		CREATE TABLE role_permissions (
			roleId BINARY(16) NOT NULL,
			permissionId BINARY(16) NOT NULL,
			PRIMARY KEY(roleId, permissionId),
			FOREIGN KEY(roleId) REFERENCES roles(roleId) ON DELETE CASCADE,
			FOREIGN KEY(permissionId) REFERENCES permissions(permissionId) ON DELETE CASCADE
		);
		`,
	);
}
async function createResourceGroups() {
	console.log('Creating resource_groups table');
	await pool.query(
		`
		CREATE TABLE resource_groups (
			groupId BINARY(16) NOT NULL,
			resourceId BINARY(16) NOT NULL,
			roleId INT NOT NULL,
			PRIMARY KEY(groupId, resourceId, roleId),
			FOREIGN KEY(groupId) REFERENCES user_groups(groupId) ON DELETE CASCADE,
			FOREIGN KEY(resourceId) REFERENCES resources(resourceId) ON DELETE CASCADE
		  );
		`,
	);
}

async function createResourceUsers() {
	console.log('Creating resource_users table');
	await pool.query(
		`
		CREATE TABLE resource_users (
			resourceId BINARY(16) NOT NULL,
			userId BINARY(16) NOT NULL,
			roleId INT NOT NULL,
			PRIMARY KEY(userId, resourceId, roleId),
			FOREIGN KEY(userId) REFERENCES users(userId) ON DELETE CASCADE,
			FOREIGN KEY(resourceId) REFERENCES resources(resourceId) ON DELETE CASCADE
		  );
		`,
	);
}

// TODO: Primary key constraints for this
async function createDeployments() {
	console.log('Creating deployments table');
	await pool.query(
		`
		CREATE TABLE deployments (
			deploymentId BINARY(16) PRIMARY KEY,
			repository VARCHAR(100) NOT NULL,
			tag VARCHAR(100) NOT NULL,
			configuration JSON,
			hostConfig JSON,
			serverId BINARY(16),
			UNIQUE(repository, tag, serverId),
			FOREIGN KEY(serverId) REFERENCES servers(serverId)
		);
		`,
	);
}


async function createDomains() {
	console.log('Creating domains table');
	await pool.query(
		`
		CREATE TABLE domains(
			domain VARCHAR(255) PRIMARY KEY,
			deploymentId BINARY(16) NOT NULL,
			port SMALLINT UNSIGNED NOT NULL,
			verified TINYINT(1) NOT NULL,
			challenge BINARY(16) NOT NULL,
			FOREIGN KEY(deploymentId) REFERENCES deployments(deploymentId)
		)
		`,
	);
}

// TODO: Server authentication details needed by deployer would go here (docker tlsverify certs).
// Also details like the server region (in the future) would go here
async function createServers() {
	console.log('Creating servers table');
	await pool.query(
		`
		CREATE TABLE servers (
			serverId BINARY(16) PRIMARY KEY,
			ip CHAR(15) NOT NULL,
			port SMALLINT UNSIGNED NOT NULL,
			UNIQUE(IP, PORT)
		);
		`,
	);
}


async function initialiseAdminGroup() {
	await createGroup({
		groupId: Buffer.from('0'.repeat(32), 'hex'),
		name: 'site-admins',
	});
}

export default async function initialise() {
	console.log('Initialising database');
	const rows = await pool.query('SHOW TABLES;');
	if (rows.length === 0) {
		console.log('No tables exist. Creating fresh');
		await createUsers();
		await createClients();
		await createResources();
		await createGroups();
		await createPermissions();
		await createRoles();
		await createRolePermissions();
		await createResourceUsers();
		await createResourceGroups();
		await createServers();
		await createDeployments();
		await createDomains();

		await initialiseAdminGroup();
		console.log('All tables created');
	}
}

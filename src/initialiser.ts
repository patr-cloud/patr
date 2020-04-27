import pool from './models/database';

async function createUsers() {
	console.log('Creating users table');
	await pool.query(
		`
		CREATE TABLE users(
			userId VARCHAR(36) PRIMARY KEY,
			username VARCHAR(80) UNIQUE NOT NULL,
			password VARCHAR(64) NOT NULL,
			email VARCHAR(64) UNIQUE NOT NULL
		  );
		`,
	);
}

async function createGroups() {
	console.log('Creating groups table');
	await pool.query(
		`
		CREATE TABLE groups(
			name VARCHAR(80) UNIQUE NOT NULL,
			groupId VARCHAR(36) PRIMARY KEY,
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
			name VARCHAR(80) NOT NULL,
			type VARCHAR(80) NOT NULL,
			resourceId VARCHAR(36) PRIMARY KEY,
			UNIQUE KEY(name, type)
		  );
		  
		`,
	);
}

async function createPermissions() {
	console.log('Creating permissions table');
	await pool.query(
		`
		CREATE TABLE permissions(
			name VARCHAR(80) NOT NULL,
			permissionId VARCHAR(36) PRIMARY KEY
		  );
		`,
	);
}

async function createRoles() {
	console.log('Creating roles table');
	await pool.query(
		`
		CREATE TABLE roles(
			name VARCHAR(80) NOT NULL,
			roleId VARCHAR(36) PRIMARY KEY,
			groupId VARCHAR(36) NULL,
			FOREIGN KEY(groupId) REFERENCES groups(groupId) ON DELETE CASCADE
		);
		`,
	);
}

async function createRolePermissions() {
	console.log('Creating role_permissions Table');
	await pool.query(
		`
		CREATE TABLE role_permissions (
			roleId VARCHAR(36) NOT NULL,
			permissionId VARCHAR(36) NOT NULL,
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
			groupId VARCHAR(36) NOT NULL,
			resourceId VARCHAR(36) NOT NULL,
			roleId VARCHAR(36) NOT NULL,
			PRIMARY KEY(groupId, resourceId, roleId),
			FOREIGN KEY(groupId) REFERENCES groups(groupId) ON DELETE CASCADE,
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
			resourceId VARCHAR(36) NOT NULL,
			userId VARCHAR(36) NOT NULL,
			roleId VARCHAR(36) NOT NULL,
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
			repository VARCHAR(36) NOT NULL,
			tag VARCHAR(36) NOT NULL,
			configuration JSON,
			serverId VARCHAR(36) NOT NULL,
			FOREIGN KEY(serverId) REFERENCES servers(serverId)
        );
        `,
	);
}

// TODO: Server authentication details needed by deployer would go here (docker tlsverify certs).
// Also details like the server region (in the future) would go here
async function createServers() {
	console.log('Create servers table');
	await pool.query(
		`
		CREATE TABLE servers (
			serverId VARCHAR(36) NOT NULL,
			ip VARCHAR(15) UNIQUE NOT NULL,
			PRIMARY KEY(serverId),
		);
		`,
	);
}

export default async function initialise() {
	console.log('Initialising database');
	const rows = await pool.query('SHOW TABLES;');
	if (rows.length === 0) {
		console.log('No tables exist. Creating fresh');
		await createUsers();
		await createResources();
		await createGroups();
		await createPermissions();
		await createRoles();
		await createRolePermissions();
		await createResourceUsers();
		await createResourceGroups();
		await createServers();
		await createDeployments();
		console.log('All tables created');
	}
}

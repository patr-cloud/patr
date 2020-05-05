import pool from './models/database';

async function createUsers() {
	console.log('Creating users table');
	await pool.query(
		`
		CREATE TABLE users(
			userId BINARY(16) PRIMARY KEY,
			username VARCHAR(80) UNIQUE NOT NULL,
			password BINARY(60) NOT NULL,
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
			name VARCHAR(80) UNIQUE NOT NULL,
			groupId BINARY(16) PRIMARY KEY,
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
			resourceId BINARY(16) PRIMARY KEY,
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
			permissionId BINARY(16) PRIMARY KEY
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
			roleId BINARY(16) PRIMARY KEY,
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
			roleId BINARY(16) NOT NULL,
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
			roleId BINARY(16) NOT NULL,
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
			repository VARCHAR(80) NOT NULL,
			tag VARCHAR(36) NOT NULL,
			configuration JSON
        );
        `,
	);
}

async function createDeploymentServers() {
	console.log('Creating deployment_servers table');
	await pool.query(
		`
        CREATE TABLE deployment_servers(
			deploymentId BINARY(16),
			serverId BINARY(16), 
			FOREIGN KEY(serverId) REFERENCES servers(serverId),
			FOREIGN KEY(deploymentId) REFERENCES deployments(deploymentId),
			UNIQUE(deploymentId, serverId)
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
            ip INTEGER UNIQUE NOT NULL,
			port SMALLINT UNSIGNED NOT NULL,
			UNIQUE(IP, PORT)
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
		await createDeploymentServers();
		console.log('All tables created');
	}
}

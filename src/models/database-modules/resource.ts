import { v4 } from 'uuid';
import pool from '../database';
import { Resource } from '../interfaces/resource';

export async function createResource(resource: Resource) {
	if (!resource.resourceId) {
		resource.resourceId = v4({}, Buffer.alloc(16));
	}
	await pool.query(
		`
		INSERT INTO
			resources(resourceId, name, type)
		VALUES
			(?, ?, ?);
		`,
		[resource.resourceId, resource.name, resource.type],
	);
}

export async function getResourceByName(resourceName: string): Promise<Resource> {
	const resources = await pool.query(
		`
		SELECT
			*
		FROM
			resources
		WHERE
			name = ?
		`,
		[resourceName],
	);

	if (resources.length === 1) {
		return resources[0];
	}
	return null;
}

export async function deleteResource(resourceId: string) {
	await pool.query(
		`
		DELETE FROM
			resource
		WHERE
			resourceId = ?
		`,
		[resourceId],
	);
}

export async function grantUserResource(userId: Buffer, resourceId: Buffer, roleId: number) {
	await pool.query(
		`
		INSERT INTO
			resource_users(resourceId, userId, roleId)
		VALUES
			(?, ?, ?)

		`,
		[resourceId, userId, roleId],
	);
}

export async function grantGroupResource(groupId: Buffer, resourceId: Buffer, roleId: number) {
	await pool.query(
		`
		INSERT INTO
			resource_groups(resourceId, groupId, roleId)
		VALUES
			(?, ?, ?)
		`,
		[resourceId, groupId, roleId],
	);
}

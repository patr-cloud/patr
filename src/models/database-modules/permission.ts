import pool from '../database';
import { checkIfRoleGrantsPermission } from './role';
import { Permission } from '../interfaces/permission';

/*
 * Takes userId, and userGroups (array of groupsIds which the user
 * is a part of)
 * Checks if the user can perform the permission 'permission' on any
 * of the resources with name resourceNames and type resourceType
* */
export default async function checkIfUserHasPermission(
	userId: Buffer,
	userGroups: Buffer[],
	resourceNames: string[],
	permission: Permission,
) {
	if (resourceNames.length === 0) {
		throw Error('No resourceNames provided to check middleware');
	}
	// First check if the permission is granted through one of the
	// user's groups

	if (userGroups.length > 0) {
		const groupsGrants = await pool.query(
			`
			SELECT
				resource_groups.roleId
			FROM
				resources,
				resource_groups
			WHERE
				resources.name IN (?) AND
				resources.resourceId = resource_groups.resourceId AND
				resource_groups.groupId IN (?)
		`,
			[resourceNames, userGroups],
		);
		// eslint-disable-next-line no-restricted-syntax
		for (const grant of groupsGrants) {
			if (checkIfRoleGrantsPermission(grant.roleId, permission)) {
				return true;
			}
		}
	}

	// If none of the userGroups grants the role, then maybe the user
	// was directly granted access to the resource
	const userGrants = await pool.query(
		`
			SELECT
				resource_users.roleId
			FROM
				resources,
				resource_users
			WHERE
				resources.name IN (?) AND
				resources.resourceId = resource_users.resourceId AND
				resource_users.userId = ?
		`,
		[resourceNames, userId],
	);

	// eslint-disable-next-line no-restricted-syntax
	for (const grant of userGrants) {
		if (checkIfRoleGrantsPermission(grant.roleId, permission)) {
			return true;
		}
	}

	return false;
}

import pool from '../database';
import { checkIfRoleGrantsPermission } from './role';
import { Permission } from '../interfaces/permission';

const siteAdminsUUID = Buffer.from('0'.repeat(32), 'hex');

/*
 * Takes userId, and userGroups (array of groupsIds which the user
 * is a part of)
 * Checks if the user can perform all the permissions 'permissions' on the
 * resource with name resourceName
 *
 * The function returns an array of booleans, representing which permissions
 * were granted and which of them weren't
 *
 * Note that resourceName can use the :: seperator to perform permission
 * checks in a hierarchial fashion.
 *
 * For example,
 *
 * check(..., 'orgName::registry::myAPI', [permissions.push, permissions.pull])
 *
 * will check if the push/pull is granted either on orgName, or orgName::registry
 * or orgName::registry::myAPI. The array of booleans returned tells which of the
 * perms were granted.
 * for eg, if [false, true] is returned, then push wasn't granted, but pull was granted
* */
export default async function checkIfUserHasPermission(
	userId: Buffer,
	userOrgs: Buffer[],
	resourceName: string,
	permissions: Permission[],
): Promise<boolean[]> {
	if (userOrgs.find((orgId) => orgId.equals(siteAdminsUUID))) {
		// If the site admin group is present in the user's groups, then
		// all permissions are granted
		return Array(permissions.length).fill(true);
	}

	const resourceNames = resourceName.split(':').map((_, i, resources) => resources.slice(0, i + 1).join('::'));
	const granted: boolean[] = Array(permissions.length).fill(false);

	// First check if the permission is granted through one of the
	// user's groups
	if (userOrgs.length > 0) {
		const orgsGrants = await pool.query(
			`
			SELECT
				resource_organizations.roleId
			FROM
				resources,
				resource_organizations
			WHERE
				resources.name IN (?) AND
				resources.resourceId = resource_organizations.resourceId AND
				resource_organizations.organizationId IN (?)
		`,
			[resourceNames, userOrgs],
		);

		let all = true;
		permissions.map((permission, i) => {
			// eslint-disable-next-line no-restricted-syntax
			for (const grant of orgsGrants) {
				if (checkIfRoleGrantsPermission(grant.roleId, permission)) {
					granted[i] = true;
					return true;
				}
			}

			all = false;
			return false;
		});

		// If all permissions were granted, just return granted, don't need
		// to make next query
		if (all) {
			return granted;
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

	permissions.map((permission, i) => {
	// eslint-disable-next-line no-restricted-syntax
		for (const grant of userGrants) {
			if (checkIfRoleGrantsPermission(grant.roleId, permission)) {
				granted[i] = true;
				return true;
			}
		}
		return false;
	});

	return granted;
}

import { v4 } from 'uuid';
import pool from '../database';
import { Group } from '../interfaces/group';
import { createResource } from './resource';


export async function createGroup(group: Group) {
	group.groupId = v4();
	await createResource({
		resourceId: group.groupId,
		name: group.name,
		type: 'group',
	});
	await pool.query(
		`
            INSERT INTO
                user_groups (UUID_TO_BIN(groupId), name)
            VALUES
                (?, ?);
            `,
		[group.groupId, group.name],
	);

	return group;
}

export async function getUserGroups(
	userId: string,
): Promise<({groupId: string, roleId: string})[]> {
	return pool.query(
		`
        SELECT
            user_groups.groupId,
			resource_users.roleId,
			user_groups.name
        FROM
            resource_users,
			user_groups
        WHERE
            resource_users.userId = ?,
			user_groups.groupId = resource_users.resourceId
        `,
		[userId],
	);
}

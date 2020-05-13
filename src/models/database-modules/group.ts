import { v4 } from 'uuid';
import pool from '../database';
import { Group } from '../interfaces/group';
import { createResource } from './resource';


export async function createGroup(group: Group) {
	if (!group.groupId) {
		group.groupId = v4({}, Buffer.alloc(16));
	}
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
	userId: Buffer,
): Promise<({groupId: Buffer, roleId: Buffer})[]> {
	const groups = await pool.query(
		`
        SELECT
            user_groups.groupId,
			resource_users.roleId,
			user_groups.name
        FROM
            resource_users,
			user_groups
        WHERE
            resource_users.userId = ? AND
			user_groups.groupId = resource_users.resourceId
        `,
		[userId],
	);

	console.log(groups);
	return groups;
}

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
                user_groups (groupId, name)
            VALUES
                (?, ?);
            `,
		[group.groupId, group.name],
	);

	return group;
}

export async function getGroupByName(groupName: string): Promise<Group> {
	const groups = await pool.query(
		`
		SELECT
			*
		FROM
			user_groups
		WHERE
			name = ?
		`,
		[groupName],
	);

	if (groups.length === 1) {
		return groups[0];
	}
	return null;
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

	return groups;
}

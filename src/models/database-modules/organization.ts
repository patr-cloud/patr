import { v4 } from 'uuid';
import pool from '../database';
import { Organization } from '../interfaces/organization';
import { createResource } from './resource';


export async function createOrganization(org: Organization) {
	if (!org.organizationId) {
		org.organizationId = v4({}, Buffer.alloc(16));
	}
	await createResource({
		resourceId: org.organizationId,
		name: org.name,
		type: 'organization',
	});
	await pool.query(
		`
            INSERT INTO
                organizations (organizationId, name)
            VALUES
                (?, ?);
            `,
		[org.organizationId, org.name],
	);

	return org;
}

export async function getOrganizationByName(organizationName: string): Promise<Organization> {
	const orgs = await pool.query(
		`
		SELECT
			*
		FROM
			organizations
		WHERE
			name = ?
		`,
		[organizationName],
	);

	if (orgs.length === 1) {
		return orgs[0];
	}
	return null;
}

export async function getUserOrgs(
	userId: Buffer,
): Promise<({organizationId: Buffer, roleId: Buffer})[]> {
	const orgs = await pool.query(
		`
        SELECT
            organizations.organizationId,
			resource_users.roleId,
			organizations.name
        FROM
            resource_users,
			organizations
        WHERE
            resource_users.userId = ? AND
			organizations.organizationId = resource_users.resourceId
        `,
		[userId],
	);

	return orgs;
}

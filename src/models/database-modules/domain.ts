import pool from '../database';
import { Domain } from '../interfaces/domain';

export async function createDomain(
	domain: Domain,
): Promise<Domain> {
	await pool.query(
		`
		INSERT INTO
			domains(deploymentId, domain, port, verified, challenge)
		VALUES
			(?, ?, ?)
		`,
		[domain.deploymentId, domain.domain, domain.port, domain.verified, domain.challenge],
	);

	return domain;
}

export async function deleteDomain(domain: string) {
	await pool.query(
		`
		DELETE FROM
			domains
		WHERE
			domain=?
		`,
		[domain],
	);
}

export async function deleteDeploymentDomains(deploymentId: Buffer) {
	await pool.query(
		`
        DELETE FROM
            domains
        WHERE
			deploymentId=?
        `,
		[deploymentId],
	);
}

export async function getDomain(domain: string): Promise<Domain> {
	const domains = await pool.query(
		`
		SELECT
			*
		FROM
			domains
		WHERE
			domain=?
		`,
		[domain],
	);

	if (domains.length === 1) {
		return domains[0];
	}
	return null;
}

export async function getDeploymentDomains(deploymentId: Buffer): Promise<Domain[]> {
	return pool.query(
		`
        SELECT
            *
        FROM
            domains
        WHERE
            deploymentId=?
        `,
		[deploymentId],
	);
}

export async function verifyDomainDB(domain: string) {
	await pool.query(
		`
        UPDATE
            domains
        SET
            verified = 1
        WHERE
            domain = ?
        `,
		[domain],
	);
}

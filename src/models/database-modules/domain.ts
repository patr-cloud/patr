import pool from '../database';
import { Domain } from '../interfaces/domain';

export async function createDomain(
	deploymentId: string,
	domain: string,
	port: number,
): Promise<Domain> {
	await pool.query(
		`
		INSERT INTO
			domains(deploymentId, domain, port)
		VALUES
			(?, ?, ?)
		`,
		[deploymentId, domain, port],
	);

	return {
		deploymentId,
		domain,
		port,
	};
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

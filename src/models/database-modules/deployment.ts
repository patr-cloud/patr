import pool from '../database';
import { Deployment } from '../interfaces/deployment';


export async function createDeployment(
	repository: string,
	tag: string,
	serverId: string,
	configuration: object,
): Promise<Deployment> {
	await pool.query(
		`
		INSERT INTO
			deployments(respository, branch, serverId, configuration)
		VALUES
			(?, ?, ?, ?)
		`,
		[repository, tag, serverId, configuration],
	);

	return {
		repository,
		tag,
		serverId,
		configuration,
	};
}

export async function getRepoDeployments(
	respository: string,
	tag: string,
): Promise<Deployment[]> {
	return pool.query(
		`
		SELECT
			*
		FROM
			deployments
		WHERE
			repository = ? AND tag = ?
		`,
		[respository, tag],
	);
}

export async function removeDeployment() {
	// To BE IMPLEMENTED
	return -1;
}

import { v4 } from 'uuid';

import pool from '../database';
import { Deployment } from '../interfaces/deployment';
import { Server } from '../interfaces/server';


export async function createDeployment(
	repository: string,
	tag: string,
	configuration: object,
	serverId: string,
): Promise<Deployment> {
	const deploymentId = v4();
	await pool.query(
		`
		INSERT INTO
			deployments(deploymentId, repository, tag, configuration, serverId)
		VALUES
			(?, ?, ?, ?)
		`,
		[deploymentId, repository, tag, JSON.stringify(configuration), serverId],
	);

	return {
		deploymentId,
		repository,
		tag,
		configuration,
		serverId,
	};
}

export function getDeploymentsById(
	deploymentId: string,
): Promise<(Deployment & Server)[]> {
	return pool.query(
		`
		SELECT
			deployments.deploymentId,
			deployments.repository,
			deployments.tag,
			deployments.configuration,
			servers.serverId,
			servers.ip,
			servers.port
		FROM
			deployments,
			servers,
		WHERE
			deployments.deploymentId = ?,
			deployments.serverId = servers.severId
		`,
		[deploymentId],
	);
}


export async function getRepoDeployments(
	respository: string,
	tag: string,
) {
	const deployments: (Deployment & Server)[] = await pool.query(
		`
		SELECT
			deployments.deploymentId,
			deployments.repository,
			deployments.tag,
			deployments.configuration,
			servers.serverId,
			servers.ip,
			servers.port
		FROM
			deployments, servers
		WHERE
            deployments.repository = ?
            AND deployments.tag = ?
			AND deployments.serverId = servers.serverId
		`,
		[respository, tag],
	);

	const deployJobs: { [deploymentId: string]: any } = {};
	deployments.forEach((d) => {
		if (deployJobs[d.deploymentId]) {
			deployJobs[d.deploymentId].servers.push(
				{
					host: d.ip,
					port: d.port,
				},
			);
		} else {
			deployJobs[d.deploymentId] = {
				repository: d.repository,
				servers: [{
					host: d.ip,
					port: d.port,
				}],
				options: JSON.parse(d.configuration as any),
			};
		}
	});

	return deployJobs;
}

export async function removeDeployment() {
	// To BE IMPLEMENTED
	return -1;
}

import { v4 } from 'uuid';

import pool from '../database';
import { Deployment } from '../interfaces/deployment';
import { Server } from '../interfaces/server';


export async function createDeployment(
	repository: string,
	tag: string,
	configuration: object,
): Promise<Deployment> {
	const deploymentId = v4();
	await pool.query(
		`
		INSERT INTO
			deployments(deploymentId, repository, tag, configuration)
		VALUES
			(?, ?, ?, ?)
		`,
		[deploymentId, repository, tag, JSON.stringify(configuration)],
	);

	return {
		deploymentId,
		repository,
		tag,
		configuration,
	};
}

export async function setDeploymentServers(deploymentId: string, serverIds: string[]) {
	const values = serverIds.map((s) => [deploymentId, s]);
	await pool.query(
		`
		INSERT INTO
			deployment_servers(deploymentId, serverId)
		VALUES
			?
		`,
		[values],
	);
}

export async function getRepoDeployments(
	respository: string,
	tag: string,
) {
	const deployments: (Deployment&Server)[] = await pool.query(
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
			deployments, servers, deployment_servers
		WHERE
            deployments.repository = ?
            AND deployments.tag = ?
			AND deployment_servers.serverId = servers.serverId
			AND deployment_servers.deploymentId = deployments.deploymentId
		`,
		[respository, tag],
	);

	const deployJobs: {[deploymentId: string]: any} = {};
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
				options: JSON.parse(d.configuration as string),
			};
		}
	});

	return deployJobs;
}

export async function removeDeployment() {
	// To BE IMPLEMENTED
	return -1;
}

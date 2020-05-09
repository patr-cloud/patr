import { v4 } from 'uuid';
import { parse } from 'url';

import pool from '../database';
import { Deployment, DeployJob, RegAuth } from '../interfaces/deployment';
import { Server } from '../interfaces/server';
import { dockerHubRegistry, privateRegistry } from '../../config/config';


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

	const deployJobs: DeployJob[] = deployments.map((deployment) => {
		let auth: RegAuth | undefined;
		const registryUrl = parse(deployment.repository).hostname;
		if (registryUrl === dockerHubRegistry.serveraddress) {
			auth = undefined; // Don't pass authentication for docker hub
		} else if (registryUrl === privateRegistry.serveraddress) {
			auth = privateRegistry;
		}
		return {
			id: deployment.deploymentId,
			image: deployment.repository,
			server: {
				host: deployment.ip,
				port: deployment.port,
			},
			auth,
			configuration: JSON.parse(deployment.configuration as any),
		};
	});

	return deployJobs;
}

export async function removeDeployment() {
	// To BE IMPLEMENTED
	return -1;
}

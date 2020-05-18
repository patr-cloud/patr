import { v4 } from 'uuid';
import { parse } from 'url';

import pool from '../database';
import { Deployment, DeployJob, RegAuth } from '../interfaces/deployment';
import { Server } from '../interfaces/server';
import { dockerHubRegistry, privateRegistry } from '../../config/config';
import { deleteDeploymentDomains } from './domain';


export async function createDeployment(
	deployment: Deployment,
): Promise<Deployment> {
	if (!deployment.deploymentId) {
		deployment.deploymentId = v4({}, Buffer.alloc(16));
	}
	await pool.query(
		`
		INSERT INTO
			deployments(deploymentId, repository, tag, configuration, serverId)
		VALUES
			(?, ?, ?, ?)
		`,
		[
			deployment.deploymentId,
			deployment.repository,
			deployment.tag,
			JSON.stringify(deployment.configuration),
			deployment.serverId,
		],
	);

	return deployment;
}

export async function updateDeploymentConfig(
	deploymentId: Buffer,
	hostConfig: Deployment['hostConfig'],
): Promise<boolean> {
	const update = await pool.query(
		`
		UPDATE
			deployments
		SET
			hostConfig = ?
		WHERE
			deploymentId = ?
		`,
		[JSON.stringify(hostConfig), deploymentId],
	);

	return !!update;
}

export async function deleteDeployment(
	deploymentId: Buffer,
) {
	await deleteDeploymentDomains(deploymentId);
	await pool.query(
		`
		DELETE FROM
			deployments
		WHERE
			deploymentId = ?
		`,
		[
			deploymentId,
		],
	);
}

export async function getDeploymentById(
	deploymentId: Buffer,
): Promise<(Deployment & Server)> {
	const deployments = await pool.query(
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
			servers
		WHERE
			deployments.deploymentId = ?,
			deployments.serverId = servers.severId
		`,
		[deploymentId],
	);

	if (deployments.length === 1) {
		return deployments[0];
	}
	return null;
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
			deployments.hostConfig,
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
		if (deployment.hostConfig) {
			deployment.configuration.HostConfig = deployment.hostConfig;
		}
		return {
			id: deployment.deploymentId.toString('hex'),
			image: deployment.repository,
			server: {
				host: deployment.ip,
				port: deployment.port,
			},
			auth,
			configuration: JSON.parse(deployment.configuration as string),
		};
	});

	return deployJobs;
}

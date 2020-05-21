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
			deployments(deploymentId, repository, tag, configuration, serverId, organizationId)
		VALUES
			(?, ?, ?, ?, ?, ?)
		`,
		[
			deployment.deploymentId,
			deployment.repository,
			deployment.tag,
			JSON.stringify(deployment.configuration),
			deployment.serverId,
			deployment.organizationId,
		],
	);

	return deployment;
}

export async function updateDeploymentConfig(
	deploymentId: Buffer,
	configuration: Deployment['configuration'],
) {
	await pool.query(
		`
		UPDATE
			deployments
		SET
			configuration = ?
		WHERE
			deploymentId = ?
		`,
		[JSON.stringify(configuration), deploymentId],
	);
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
			*
		FROM
			deployments,
			servers
		WHERE
			deployments.deploymentId = ? AND
			deployments.serverId = servers.serverId
		`,
		[deploymentId],
	);

	if (deployments.length === 1) {
		if (deployments[0].configuration) {
			deployments[0].configuration = JSON.parse(deployments[0].configuration);
		}
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
		const registryUrl = deployment.repository.slice(0, deployment.repository.indexOf('/'));
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
			image: `${deployment.repository}:${deployment.tag}`,
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

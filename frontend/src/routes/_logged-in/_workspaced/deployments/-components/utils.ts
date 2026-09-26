import { GetDeploymentInfoResponse, UpdateDeploymentRequest } from "~/bindings";

/**
 * Builds the body for a deployment update.
 *
 * The API takes the whole deployment on every update, so a tab that edits one
 * field still has to send the rest back untouched. `machineType` is immutable
 * but required by the request shape, so it is carried over; the registry can't
 * change after create, so it isn't part of the request at all.
 */
export const toUpdateRequest = (info: GetDeploymentInfoResponse): UpdateDeploymentRequest => ({
	name: info.name,
	imageTag: info.imageTag,
	runner: info.runner,
	machineType: info.machineType,
	deployOnPush: info.deployOnPush,
	minHorizontalScale: info.minHorizontalScale,
	maxHorizontalScale: info.maxHorizontalScale,
	ports: info.ports,
	environmentVariables: info.environmentVariables,
	startupProbe: info.startupProbe,
	livenessProbe: info.livenessProbe,
	configMounts: info.configMounts,
	volumes: info.volumes,
});

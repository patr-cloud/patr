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

/**
 * Whether two versions of a deployment would send the same update. Maps are
 * compared with their keys sorted, so one rebuilt in a different order (the env
 * editor re-emitting its rows, say) doesn't read as a change. A field that is
 * missing, `null` or an empty map counts as the same thing: the API leaves
 * empty maps out of its response, while the editors hand back `{}`.
 */
export const isSameUpdate = (a: GetDeploymentInfoResponse, b: GetDeploymentInfoResponse): boolean => {
	const isEmptyMap = (value: unknown) =>
		typeof value === "object" && value !== null && !Array.isArray(value) && Object.keys(value).length === 0;
	const canonical = (info: GetDeploymentInfoResponse) =>
		JSON.stringify(toUpdateRequest(info), (_, value) =>
			value && typeof value === "object" && !Array.isArray(value)
				? Object.fromEntries(
						Object.entries(value)
							.filter(([, field]) => field !== null && field !== undefined && !isEmptyMap(field))
							.sort(([x], [y]) => x.localeCompare(y))
					)
				: value
		);
	return canonical(a) === canonical(b);
};

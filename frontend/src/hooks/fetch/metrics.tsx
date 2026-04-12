import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import type { GetDeploymentMetricResponse, GetRunnerMetricsResponse, MetricDataPoint } from "~/bindings";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { deploymentKeys, runnerKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useRunnerMetricsQuery = (
	runnerId: Accessor<string>,
	metricNames: string[],
	interval: Accessor<string>
) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = runnerId();
		const intervalVal = interval();
		return {
			queryKey: runnerKeys.metrics(wsId ?? "", id, intervalVal),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			queryFn: async () => {
				let failCount = 0;
				const results = await Promise.all(
					metricNames.map(async (metric) => {
						const resp = await httpRequest<GetRunnerMetricsResponse>(
							`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${id}/metrics/${metric}?interval=${intervalVal}.0`,
							{ method: "GET" }
						);
						if (!resp.ok) {
							console.error(`Failed to fetch runner metric ${metric}:`, resp.data.error);
							failCount++;
							return [metric, []] as const;
						}
						return [metric, resp.data.dataPoints] as const;
					})
				);
				if (failCount === metricNames.length) {
					throw new Error("Failed to fetch metrics");
				}
				return Object.fromEntries(results) as Record<string, MetricDataPoint[]>;
			},
		};
	});
};

export const useDeploymentMetricsQuery = (
	deploymentId: Accessor<string>,
	metricNames: string[],
	interval: Accessor<string>
) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = deploymentId();
		const intervalVal = interval();
		return {
			queryKey: deploymentKeys.metrics(wsId ?? "", id, intervalVal),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			queryFn: async () => {
				let failCount = 0;
				const results = await Promise.all(
					metricNames.map(async (metric) => {
						const resp = await httpRequest<GetDeploymentMetricResponse>(
							`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${id}/metrics/${metric}?interval=${intervalVal}.0`,
							{ method: "GET" }
						);
						if (!resp.ok) {
							console.error(`Failed to fetch deployment metric ${metric}:`, resp.data.error);
							failCount++;
							return [metric, []] as const;
						}
						return [metric, resp.data.dataPoints] as const;
					})
				);
				if (failCount === metricNames.length) {
					throw new Error("Failed to fetch metrics");
				}
				return Object.fromEntries(results) as Record<string, MetricDataPoint[]>;
			},
		};
	});
};

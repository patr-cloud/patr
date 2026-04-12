import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import type { MetricDataPoint } from "~/bindings";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { deploymentKeys, runnerKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

const fetchMetrics = async (baseUrl: string, metricNames: string[], interval: string) => {
	let failCount = 0;
	const results = await Promise.all(
		metricNames.map(async (metric) => {
			const resp = await httpRequest<{ dataPoints: MetricDataPoint[] }>(
				`${baseUrl}/metrics/${metric}?interval=${interval}.0`,
				{ method: "GET" }
			);
			if (!resp.ok) {
				console.error(`Failed to fetch metric ${metric}:`, resp.data.error);
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
};

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
			queryFn: () =>
				fetchMetrics(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${id}`,
					metricNames,
					intervalVal
				),
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
			queryFn: () =>
				fetchMetrics(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${id}`,
					metricNames,
					intervalVal
				),
		};
	});
};

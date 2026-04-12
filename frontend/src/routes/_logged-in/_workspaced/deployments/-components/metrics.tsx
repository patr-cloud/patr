import { Show, For, createMemo, createResource, createSignal, ErrorBoundary, Suspense } from "solid-js";
import type { GetDeploymentMetricResponse, MetricDataPoint } from "~/bindings";
import { InputDropdown } from "~/components";
import MetricCard, { INTERVALS, type ChartDef } from "~/components/metric-card";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface DeploymentMetricsProps {
	deploymentId: string;
}

const CHARTS: ChartDef[] = [
	{
		title: "Requests per Second",
		unit: "req/s",
		yMin: 0,
		ySuggestedMax: 10,
		series: [{ metric: "ingress_rps", label: "RPS", color: "#3b82f6" }],
	},
	{
		title: "Error Rate",
		unit: "err/s",
		yMin: 0,
		ySuggestedMax: 1,
		series: [{ metric: "ingress_error_rate", label: "Errors", color: "#ef4444" }],
	},
	{
		title: "Latency (P95)",
		unit: "ms",
		yMin: 0,
		ySuggestedMax: 100,
		series: [{ metric: "ingress_latency_p95", label: "P95", color: "#f59e0b", transform: (v) => v * 1000 }],
	},
	{
		title: "CPU",
		unit: "%",
		yMin: 0,
		yMax: 100,
		series: [{ metric: "container_cpu_usage", label: "Usage", color: "#8b5cf6" }],
	},
	{
		title: "Memory",
		unit: "MB",
		yMin: 0,
		series: [
			{
				metric: "container_memory_used",
				label: "Used",
				color: "#a855f7",
				transform: (v) => v / 1024 / 1024,
			},
			{
				metric: "container_memory_limit",
				label: "Limit",
				color: "#6b7280",
				transform: (v) => v / 1024 / 1024,
			},
		],
	},
	{
		title: "Network I/O",
		unit: "KB/s",
		yMin: 0,
		ySuggestedMax: 10,
		series: [
			{ metric: "container_network_rx", label: "RX", color: "#06b6d4", transform: (v) => v / 1024 },
			{ metric: "container_network_tx", label: "TX", color: "#ec4899", transform: (v) => v / 1024 },
		],
	},
];

const METRIC_NAMES = [...new Set(CHARTS.flatMap((c) => c.series.map((s) => s.metric)))];

const DeploymentMetrics = (props: DeploymentMetricsProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const [intervalSeconds, setIntervalSeconds] = createSignal("3600");

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), props.deploymentId, intervalSeconds()] as const;
	});

	const [metricsData] = createResource(fetchParams, async ([auth, wsId, deploymentId, interval]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !deploymentId) {
			return undefined;
		}
		const results = await Promise.all(
			METRIC_NAMES.map(async (metric) => {
				const resp = await httpRequest<GetDeploymentMetricResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${deploymentId}/metrics/${metric}?interval=${interval}.0`,
					{ method: "GET" }
				);
				if (!resp.ok) {
					console.error(`Failed to fetch deployment metric ${metric}:`, resp.data.error);
					return [metric, []] as const;
				}
				return [metric, resp.data.dataPoints] as const;
			})
		);
		return Object.fromEntries(results) as Record<string, MetricDataPoint[]>;
	});

	return (
		<div class="flex flex-col gap-lg">
			{/* Interval selector */}
			<div class="flex items-center justify-end gap-sm">
				<Show when={metricsData.loading}>
					<div class="w-3.5 h-3.5 border-2 border-primary/40 border-t-primary rounded-full animate-spin" />
				</Show>
				<span class="text-xs text-grey">Interval</span>
				<div class="w-40">
					<InputDropdown
						options={INTERVALS}
						value={intervalSeconds()}
						onSelect={(val) => setIntervalSeconds(val)}
						placeholder="Select interval"
						styleVariant="dark"
					/>
				</div>
			</div>

			{/* Charts grid */}
			<ErrorBoundary
				fallback={(err, reset) => (
					<div class="rounded-sm border border-error/30 bg-error/5 p-lg text-center">
						<p class="text-sm text-error mb-xs">Failed to load metrics</p>
						<p class="text-xxs text-grey mb-md">{err.message}</p>
						<button class="text-primary text-xs cursor-pointer bg-transparent border-none" onClick={reset}>
							Retry
						</button>
					</div>
				)}
			>
				<Suspense
					fallback={
						<div class="grid grid-cols-1 lg:grid-cols-2 gap-lg">
							<For each={CHARTS}>
								{() => (
									<div
										class="rounded-sm border border-border-color bg-secondary-light animate-pulse"
										style={{ height: "310px" }}
									/>
								)}
							</For>
						</div>
					}
				>
					<div class="grid grid-cols-1 lg:grid-cols-2 gap-lg">
						<For each={CHARTS}>
							{(chart, index) => (
								<div
									class={
										index() === CHARTS.length - 1 && CHARTS.length % 2 !== 0 ? "lg:col-span-2" : ""
									}
								>
									<MetricCard chart={chart} data={metricsData()} />
								</div>
							)}
						</For>
					</div>
				</Suspense>
			</ErrorBoundary>
		</div>
	);
};

export default DeploymentMetrics;

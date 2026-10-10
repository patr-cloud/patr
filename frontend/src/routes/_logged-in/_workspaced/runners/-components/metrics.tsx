import { Show, For, createSignal, ErrorBoundary } from "solid-js";
import { InputDropdown } from "~/components";
import MetricCard, { INTERVALS, type ChartDef } from "~/components/metric-card";
import { useRunnerMetricsQuery } from "~/hooks/fetch";

interface RunnerMetricsProps {
	runnerId: string;
}

const CHARTS: ChartDef[] = [
	{
		title: "CPU",
		unit: "%",
		yMin: 0,
		yMax: 100,
		series: [{ metric: "system_cpu_usage", label: "Usage", color: "#3b82f6" }],
	},
	{
		title: "Memory",
		unit: "%",
		yMin: 0,
		yMax: 100,
		series: [{ metric: "system_memory_usage", label: "Usage", color: "#a855f7" }],
	},
	{
		title: "Disk I/O",
		unit: "KB/s",
		yMin: 0,
		ySuggestedMax: 10,
		series: [
			{ metric: "system_disk_read_bytes", label: "Read", color: "#10b981", transform: (v) => v / 1024 },
			{ metric: "system_disk_written_bytes", label: "Write", color: "#f59e0b", transform: (v) => v / 1024 },
		],
	},
	{
		title: "Disk",
		unit: "%",
		yMin: 0,
		yMax: 100,
		series: [{ metric: "system_disk_usage", label: "Usage", color: "#ef4444" }],
	},
	{
		title: "Network",
		unit: "KB/s",
		yMin: 0,
		ySuggestedMax: 10,
		series: [
			{ metric: "system_network_rx", label: "RX", color: "#06b6d4", transform: (v) => v / 1024 },
			{ metric: "system_network_tx", label: "TX", color: "#ec4899", transform: (v) => v / 1024 },
		],
	},
];

const METRIC_NAMES = [...new Set(CHARTS.flatMap((c) => c.series.map((s) => s.metric)))];

const RunnerMetrics = (props: RunnerMetricsProps) => {
	const [intervalSeconds, setIntervalSeconds] = createSignal("3600");

	const metricsQuery = useRunnerMetricsQuery(() => props.runnerId, METRIC_NAMES, intervalSeconds);

	return (
		<div class="flex flex-col gap-lg">
			{/* Interval selector */}
			<div class="flex items-center justify-end gap-sm">
				<Show when={metricsQuery.isFetching}>
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
				<Show
					when={!metricsQuery.isPending}
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
									<MetricCard chart={chart} data={metricsQuery.data} isError={metricsQuery.isError} />
								</div>
							)}
						</For>
					</div>
				</Show>
			</ErrorBoundary>
		</div>
	);
};

export default RunnerMetrics;

import { Show, For, createSignal, ErrorBoundary } from "solid-js";
import { lt } from "semver";
import { FiAlertTriangle } from "solid-icons/fi";
import {
	CopyableField,
	CopyableFieldVariant,
	Input,
	InputDropdown,
	InputLabel,
	InputType,
	Tooltip,
} from "~/components";
import MetricCard, { INTERVALS, type ChartDef } from "~/components/metric-card";
import { useRunnerMetricsQuery } from "~/hooks/fetch";
import { formatDate, formatRelativeTime } from "~/utils/func";

interface RunnerMetricsProps {
	runnerId: string;
	version: string;
	connected: boolean;
	lastSeen: Date | null;
	apiVersion: string | undefined;
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

	// '0.0.0' is the backfill sentinel for rows that existed before version
	// tracking or never reported a version.
	const versionUnknown = () => props.version === "0.0.0";
	// Only flag outdated once we know the runner has actually connected at
	// least once — fresh-never-connected rows sit at the sentinel and there's
	// nothing for the operator to act on.
	const outdated = () =>
		!versionUnknown() && !!props.lastSeen && !!props.apiVersion && lt(props.version, props.apiVersion);

	return (
		<div class="flex flex-col gap-lg">
			{/* Identity block */}
			<div class="flex flex-col space-y-4 py-lg">
				<div class="flex items-center gap-4">
					<InputLabel parentClass="flex-2" for="runner-id" label="ID" />
					<CopyableField
						variant={CopyableFieldVariant.Input}
						value={props.runnerId}
						buttonPosition="start"
						class="flex-10"
					/>
				</div>

				<div class="flex flex-col gap-xs">
					<div class="flex items-center gap-4">
						<InputLabel parentClass="flex-2" for="runner-version" label="Version" />
						<Input
							value={
								versionUnknown()
									? "Unknown"
									: outdated()
										? `${props.version} (update available)`
										: props.version
							}
							disabled={true}
							class="flex-10"
							innerClass={outdated() ? "disabled:text-warning" : ""}
							name="runner-version"
							placeholder="Runner version"
							type={InputType.Text}
							startIcon={
								outdated()
									? () => (
											<FiAlertTriangle
												class="text-warning shrink-0 ml-md mr-sm"
												size={14}
												aria-label="Update available"
											/>
										)
									: undefined
							}
						/>
					</div>
					<Show when={outdated()}>
						<div class="flex items-center gap-4">
							<div class="flex-2" />
							<p class="flex-10 text-xs text-grey">
								Run <code class="text-white font-log">patr upgrade</code> on the runner host to update.
							</p>
						</div>
					</Show>
				</div>

				<div class="flex items-center gap-4">
					<InputLabel parentClass="flex-2" for="runner-last-connected" label="Last Connected" />
					<Tooltip content={props.lastSeen ? formatDate(props.lastSeen) : ""} class="flex-10 text-white">
						<Input
							value={
								props.connected ? "Now" : props.lastSeen ? formatRelativeTime(props.lastSeen) : "Never"
							}
							disabled={true}
							class="w-full"
							name="runner-last-connected"
							placeholder="Last connected"
							type={InputType.Text}
						/>
					</Tooltip>
				</div>
			</div>

			<hr class="border-border-color" />

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

import {
	Show,
	For,
	createMemo,
	createResource,
	createSignal,
	ErrorBoundary,
	onCleanup,
	onMount,
	Suspense,
} from "solid-js";
import { Chart, registerables } from "chart.js";
import { GetRunnerMetricsResponse } from "~/bindings";
import { InputDropdown, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

Chart.register(...registerables);

interface RunnerMetricsProps {
	runnerId: string;
}

interface DataPoint {
	timestamp: string;
	value: string;
}

const INTERVALS = [
	{ label: "1 hour", value: "3600" },
	{ label: "6 hours", value: "21600" },
	{ label: "12 hours", value: "43200" },
	{ label: "24 hours", value: "86400" },
	{ label: "2 days", value: "172800" },
	{ label: "7 days", value: "604800" },
];

interface ChartDef {
	title: string;
	unit: string;
	series: {
		field: string;
		label: string;
		color: string;
		transform?: (v: number) => number;
	}[];
}

const CHARTS: ChartDef[] = [
	{
		title: "CPU",
		unit: "%",
		series: [{ field: "cpuUsage", label: "Usage", color: "#3b82f6" }],
	},
	{
		title: "Memory",
		unit: "%",
		series: [{ field: "memoryUsage", label: "Usage", color: "#a855f7" }],
	},
	{
		title: "Disk I/O",
		unit: "KB/s",
		series: [
			{ field: "diskReadBytes", label: "Read", color: "#10b981", transform: (v) => v / 1024 },
			{ field: "diskWrittenBytes", label: "Write", color: "#f59e0b", transform: (v) => v / 1024 },
		],
	},
	{
		title: "Disk",
		unit: "%",
		series: [{ field: "diskUsage", label: "Usage", color: "#ef4444" }],
	},
	{
		title: "Network",
		unit: "KB/s",
		series: [
			{ field: "networkUsageRx", label: "RX", color: "#06b6d4", transform: (v) => v / 1024 },
			{ field: "networkUsageTx", label: "TX", color: "#ec4899", transform: (v) => v / 1024 },
		],
	},
];

const currentValue = (points: DataPoint[] | undefined, transform?: (v: number) => number): string => {
	if (!points || points.length === 0) return "--";
	const raw = parseFloat(points[points.length - 1].value) || 0;
	const val = transform ? transform(raw) : raw;
	return val < 10 ? val.toFixed(2) : val.toFixed(1);
};

const MetricCard = (props: { chart: ChartDef; data: GetRunnerMetricsResponse | undefined }) => {
	let canvasRef!: HTMLCanvasElement;
	let chartInstance: Chart | undefined;

	const allData = () => props.data as Record<string, DataPoint[]> | undefined;

	const hasData = () => {
		const d = allData();
		if (!d) return false;
		return props.chart.series.some((s) => {
			const pts = d[s.field];
			return pts && pts.length > 0;
		});
	};

	const primaryCurrent = () => {
		const s = props.chart.series[0];
		return currentValue(allData()?.[s.field], s.transform);
	};

	onMount(() => {
		const d = allData();
		const primaryPoints = d?.[props.chart.series[0].field] || [];
		const labels = primaryPoints.map((p: DataPoint) => {
			const date = new Date(p.timestamp);
			return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
		});

		const datasets = props.chart.series.map((s) => {
			const pts = d?.[s.field] || [];
			const transform = s.transform || ((v: number) => v);
			return {
				label: s.label,
				data: pts.map((p: DataPoint) => transform(parseFloat(p.value) || 0)),
				borderColor: s.color,
				backgroundColor: s.color + "18",
				borderWidth: 1.5,
				pointRadius: 0,
				pointHoverRadius: 3,
				fill: true,
				tension: 0.4,
			};
		});

		chartInstance = new Chart(canvasRef, {
			type: "line",
			data: { labels, datasets },
			options: {
				responsive: true,
				maintainAspectRatio: false,
				interaction: {
					mode: "index",
					intersect: false,
				},
				plugins: {
					legend: { display: false },
					tooltip: {
						backgroundColor: "#1a1233",
						titleColor: "#ffffffac",
						bodyColor: "#ffffff",
						borderColor: "#414245",
						borderWidth: 1,
						padding: 10,
						titleFont: { family: "Poppins", size: 11 },
						bodyFont: { family: "SUSE Mono", size: 12 },
						cornerRadius: 6,
						displayColors: true,
						boxPadding: 4,
					},
				},
				scales: {
					x: {
						display: true,
						ticks: {
							color: "#ffffff40",
							font: { family: "Poppins", size: 10 },
							maxTicksLimit: 6,
							maxRotation: 0,
						},
						grid: { display: false },
						border: { display: false },
					},
					y: {
						display: true,
						ticks: {
							color: "#ffffff40",
							font: { family: "SUSE Mono", size: 10 },
							maxTicksLimit: 5,
							padding: 8,
						},
						grid: {
							color: "#ffffff08",
							lineWidth: 1,
						},
						border: { display: false },
					},
				},
			},
		});
	});

	onCleanup(() => chartInstance?.destroy());

	return (
		<div class="group relative rounded-xs border border-border-color bg-secondary-light overflow-hidden transition-colors duration-200 hover:border-primary/30">
			{/* Header bar */}
			<div class="flex items-center justify-between px-md py-sm border-b border-border-color">
				<div class="flex items-center gap-xs">
					<span class="text-sm font-medium text-white">{props.chart.title}</span>
					<Show when={props.chart.series.length > 1}>
						<div class="flex items-center gap-xs ml-xs">
							<For each={props.chart.series}>
								{(s) => (
									<div class="flex items-center gap-1">
										<span
											class="inline-block w-2 h-2 rounded-full"
											style={{ "background-color": s.color }}
										/>
										<span class="text-xs text-grey">{s.label}</span>
									</div>
								)}
							</For>
						</div>
					</Show>
				</div>
				<div class="flex items-baseline gap-xxs">
					<span class="text-xxs text-grey/50 mr-xxs">latest</span>
					<span class="font-log text-md text-white tabular-nums">{primaryCurrent()}</span>
					<span class="text-xs text-grey">{props.chart.unit}</span>
				</div>
			</div>

			{/* Chart area */}
			<div class="relative px-sm pt-sm pb-xs" style={{ height: "260px" }}>
				<Show
					when={hasData()}
					fallback={
						<div class="absolute inset-0 flex items-center justify-center">
							<div class="flex flex-col items-center gap-sm">
								<div class="flex gap-1 items-end">
									<For each={[...Array(16)]}>
										{(_, i) => (
											<div
												class="w-1.5 rounded-full bg-secondary-medium"
												style={{
													height: `${20 + Math.sin(i() * 0.7) * 16}px`,
													opacity: 0.3 + Math.sin(i() * 0.5) * 0.15,
												}}
											/>
										)}
									</For>
								</div>
								<span class="text-xs text-grey/50">Awaiting data</span>
							</div>
						</div>
					}
				>
					<canvas ref={canvasRef} />
				</Show>
			</div>
		</div>
	);
};

const RunnerMetrics = (props: RunnerMetricsProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const [intervalSeconds, setIntervalSeconds] = createSignal("3600");

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), props.runnerId, intervalSeconds()] as const;
	});

	const [metricsData] = createResource(fetchParams, async ([auth, wsId, runnerId, interval]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !runnerId) {
			return undefined;
		}
		const response = await httpRequest<GetRunnerMetricsResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${runnerId}/metrics?interval=${interval}.0`,
			{
				method: "GET",
			}
		);
		if (!response.ok) {
			console.error("Failed to fetch runner metrics:", response.data.error);
			toast("Failed to fetch runner metrics", "error");
			return undefined;
		}
		return response.data;
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
						<button class="btn btn-plain text-xs" onClick={reset}>
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
						<For each={CHARTS}>{(chart) => <MetricCard chart={chart} data={metricsData()} />}</For>
					</div>
				</Suspense>
			</ErrorBoundary>
		</div>
	);
};

export default RunnerMetrics;

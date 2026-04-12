import { Show, For, createEffect, onCleanup } from "solid-js";
import { Chart, registerables } from "chart.js";
import { parseDate } from "~/utils/func";
import type { MetricDataPoint } from "~/bindings";

Chart.register(...registerables);

export interface SeriesDef {
	/** The metric name used as key in the data record */
	metric: string;
	/** Display label for the series */
	label: string;
	/** CSS color for the series line */
	color: string;
	/** Optional transform applied to raw numeric values */
	transform?: (v: number) => number;
}

export interface ChartDef {
	title: string;
	unit: string;
	yMin: number;
	yMax?: number;
	ySuggestedMax?: number;
	series: SeriesDef[];
}

export const INTERVALS = [
	{ label: "1 hour", value: "3600" },
	{ label: "6 hours", value: "21600" },
	{ label: "12 hours", value: "43200" },
	{ label: "24 hours", value: "86400" },
	{ label: "2 days", value: "172800" },
	{ label: "7 days", value: "604800" },
];

export const currentValue = (points: MetricDataPoint[] | undefined, transform?: (v: number) => number): string => {
	if (!points || points.length === 0) return "--";
	const raw = parseFloat(points[points.length - 1].value) || 0;
	const val = transform ? transform(raw) : raw;
	return val < 10 ? val.toFixed(2) : val.toFixed(1);
};

const MetricCard = (props: { chart: ChartDef; data: Record<string, MetricDataPoint[]> | undefined }) => {
	let canvasRef!: HTMLCanvasElement;
	let chartInstance: Chart | undefined;

	const hasData = () => {
		const d = props.data;
		if (!d) return false;
		return props.chart.series.some((s) => {
			const pts = d[s.metric];
			return pts && pts.length > 0;
		});
	};

	const primaryCurrent = () => {
		const s = props.chart.series[0];
		return currentValue(props.data?.[s.metric], s.transform);
	};

	const buildChart = () => {
		const d = props.data;
		const primaryPoints = d?.[props.chart.series[0].metric] || [];
		const labels = primaryPoints.map((p) => {
			const date = parseDate(p.timestamp);
			return date?.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) ?? "";
		});

		const datasets = props.chart.series.map((s) => {
			const pts = d?.[s.metric] || [];
			const transform = s.transform || ((v: number) => v);
			return {
				label: s.label,
				data: pts.map((p) => transform(parseFloat(p.value) || 0)),
				borderColor: s.color,
				backgroundColor: s.color + "18",
				borderWidth: 1.5,
				pointRadius: 0,
				pointHoverRadius: 3,
				fill: true,
				tension: 0.4,
			};
		});

		return { labels, datasets };
	};

	createEffect(() => {
		if (!hasData() || !canvasRef) return;
		const { labels, datasets } = buildChart();

		if (chartInstance) {
			chartInstance.data.labels = labels;
			chartInstance.data.datasets = datasets;
			chartInstance.update();
			return;
		}

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
						min: props.chart.yMin,
						max: props.chart.yMax,
						suggestedMax: props.chart.ySuggestedMax,
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

export default MetricCard;

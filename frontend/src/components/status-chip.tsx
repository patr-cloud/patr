import type { ResourceTypeName } from "~/bindings";
import StatusDot from "~/components/status-dot";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface StatusChipProps {
	/** The status text to display */
	status: string;
	/**
	 * The type of resource the status belongs to. A deployment is `unreachable` when its
	 * runner is offline, so it reads "Runner offline" rather than the runner's own "Unreachable".
	 */
	resourceType?: ResourceTypeName;
	/** Size variant - "sm" for tables/inline, "md" for headers/detail pages. Defaults to "sm" */
	size?: "sm" | "md";
	/** Additional classes for the chip */
	class?: MaybeAccessor<string | undefined>;
}

/** Labels for statuses that aren't a single word, matching the CLI's. */
const LABELS: Record<string, string> = {
	imagePullFailed: "Image pull failed",
	invalidArchitecture: "Invalid architecture",
	healthCheckFailing: "Health check failing",
};

/** A colored status badge with dot indicator for displaying deployment/resource status. */
const StatusChip = (props: StatusChipProps) => {
	const size = () => props.size ?? "sm";

	const config = () => {
		switch (props.status) {
			case "running":
			case "connected":
			case "verified":
				return {
					bg: "bg-success/10",
					border: "border-success/20",
					text: "text-success",
				};
			case "errored":
			case "crashing":
			case "healthCheckFailing":
			case "imagePullFailed":
			case "invalidArchitecture":
				return {
					bg: "bg-error/10",
					border: "border-error/20",
					text: "text-error",
				};
			case "stopped":
			case "stopping":
			case "unreachable":
			case "unknown":
			case "not verified":
				return {
					bg: "bg-white/6",
					border: "border-white/10",
					text: "text-grey",
				};
			default:
				return {
					bg: "bg-warning/10",
					border: "border-warning/20",
					text: "text-warning",
				};
		}
	};

	const label = () => {
		if (props.resourceType === "deployment" && props.status === "unreachable") return "Runner offline";
		return LABELS[props.status] ?? props.status.charAt(0).toUpperCase() + props.status.slice(1);
	};

	const chipAnimation = () => {
		switch (props.status) {
			case "errored":
			case "crashing":
			case "healthCheckFailing":
			case "imagePullFailed":
			case "invalidArchitecture":
				return { animation: "throb-glow 1.5s ease-in-out infinite" };
			default:
				return undefined;
		}
	};

	const sizeClasses = () => {
		switch (size()) {
			case "md":
				return "h-10 text-sm px-md";
			case "sm":
			default:
				return "text-xs px-2.5 py-1";
		}
	};

	return (
		<span
			class={`inline-flex items-center gap-1.5 font-medium rounded-xs border tracking-wide ${sizeClasses()} ${config().bg} ${config().border} ${config().text} ${get(props.class) ?? ""}`}
			style={chipAnimation()}
		>
			<StatusDot status={props.status} size={size()} />
			{label()}
		</span>
	);
};

export default StatusChip;

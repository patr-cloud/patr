import StatusDot from "~/components/status-dot";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface StatusChipProps {
	/** The status text to display */
	status: string;
	/** Size variant - "sm" for tables/inline, "md" for headers/detail pages. Defaults to "sm" */
	size?: "sm" | "md";
	/** Additional classes for the chip */
	class?: MaybeAccessor<string | undefined>;
}

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
				return {
					bg: "bg-error/10",
					border: "border-error/20",
					text: "text-error",
				};
			case "stopped":
			case "unreachable":
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

	const chipAnimation = () => {
		if (props.status === "errored") {
			return { animation: "throb-glow 1.5s ease-in-out infinite" };
		}
		return undefined;
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
			class={`inline-flex items-center gap-1.5 font-medium rounded-xs border capitalize tracking-wide ${sizeClasses()} ${config().bg} ${config().border} ${config().text} ${get(props.class) ?? ""}`}
			style={chipAnimation()}
		>
			<StatusDot status={props.status} size={size()} />
			{props.status}
		</span>
	);
};

export default StatusChip;

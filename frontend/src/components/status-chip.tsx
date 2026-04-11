interface StatusChipProps {
	/** The status text to display */
	status: string;
	/** Size variant - "sm" for tables/inline, "md" for headers/detail pages. Defaults to "sm" */
	size?: "sm" | "md";
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
					dot: "bg-success shadow-[0_0_6px_rgba(71,201,108,0.5)]",
				};
			case "errored":
				return {
					bg: "bg-error/10",
					border: "border-error/20",
					text: "text-error",
					dot: "bg-error shadow-[0_0_6px_rgba(214,43,54,0.5)]",
				};
			case "stopped":
			case "unreachable":
			case "not verified":
				return {
					bg: "bg-white/6",
					border: "border-white/10",
					text: "text-grey",
					dot: "bg-grey opacity-50",
				};
			default:
				return {
					bg: "bg-warning/10",
					border: "border-warning/20",
					text: "text-warning",
					dot: "bg-warning",
				};
		}
	};

	const dotAnimation = () => {
		if (props.status === "deploying") {
			return { animation: "pulse-dot 1.5s ease-in-out infinite" };
		}
		return undefined;
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

	const dotSize = () => (size() === "md" ? "w-1.75 h-1.75" : "w-1.5 h-1.5");

	return (
		<span
			class={`inline-flex items-center gap-1.5 font-medium rounded-xs border capitalize tracking-wide ${sizeClasses()} ${config().bg} ${config().border} ${config().text}`}
			style={chipAnimation()}
		>
			<span class={`rounded-full shrink-0 ${dotSize()} ${config().dot}`} style={dotAnimation()} />
			{props.status}
		</span>
	);
};

export default StatusChip;

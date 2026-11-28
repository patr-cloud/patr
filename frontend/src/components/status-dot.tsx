interface StatusDotProps {
	/** The status to show the color of */
	status: string;
	/** Size variant - "sm" for tables/inline, "md" for headers/detail pages. Defaults to "sm" */
	size?: "sm" | "md";
}

/** A colored dot showing a deployment/resource status. */
const StatusDot = (props: StatusDotProps) => {
	const color = () => {
		switch (props.status) {
			case "running":
			case "connected":
			case "verified":
				return "bg-success shadow-[0_0_6px_rgba(71,201,108,0.5)]";
			case "errored":
			case "crashing":
			case "healthCheckFailing":
			case "imagePullFailed":
			case "invalidArchitecture":
				return "bg-error shadow-[0_0_6px_rgba(214,43,54,0.5)]";
			case "stopped":
			case "stopping":
			case "unreachable":
			case "unknown":
			case "not verified":
				return "bg-grey opacity-50";
			default:
				return "bg-warning";
		}
	};

	const animation = () => {
		switch (props.status) {
			case "deploying":
			case "stopping":
				return { animation: "pulse-dot 1.5s ease-in-out infinite" };
			default:
				return undefined;
		}
	};

	const size = () => (props.size === "md" ? "w-1.75 h-1.75" : "w-1.5 h-1.5");

	return <span class={`rounded-full shrink-0 ${size()} ${color()}`} style={animation()} />;
};

export default StatusDot;

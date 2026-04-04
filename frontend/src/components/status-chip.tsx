interface StatusChipProps {
	/** The status text to display */
	status: string;
}

/** A colored pill chip for displaying deployment/resource status. */
const StatusChip = (props: StatusChipProps) => {
	const colorClass = () => {
		switch (props.status) {
			case "running":
			case "connected":
				return "bg-success/15 text-success";
			case "stopped":
			case "unreachable":
				return "bg-grey/15 text-grey";
			case "errored":
				return "bg-error/15 text-error";
			default:
				return "bg-warning/15 text-warning";
		}
	};

	return <span class={`text-xs px-xs py-0.5 rounded-xl ${colorClass()}`}>{props.status}</span>;
};

export default StatusChip;

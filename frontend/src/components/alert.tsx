import { FiAlertCircle, FiCheckCircle } from "solid-icons/fi";

interface AlertProps {
	/** The alert message to display */
	message: string;
	/** The type of alert */
	type: "error" | "success" | "warning";
	/** Additional Classes to apply */
	class?: string;
	/**
	 * Hold the alert to a single 16px line, ending in an ellipsis when the
	 * message is too long, with the full text on hover. For alerts sitting
	 * inline beside other controls; leave it off where the message is the
	 * point and must be readable in full.
	 */
	truncate?: boolean;
	/** Leave out the leading icon, for alerts where the colour alone says enough. */
	hideIcon?: boolean;
}

const Alert = (props: AlertProps) => {
	return (
		<span
			class={`${props.class ?? ""} text-white flex items-center gap-2 justify-start ${
				props.truncate ? "min-w-0 max-h-4" : ""
			}`}
			title={props.truncate ? props.message : undefined}
		>
			{!props.hideIcon && props.type === "error" && (
				<FiAlertCircle size={16} class={`text-error ${props.truncate ? "shrink-0" : ""}`} />
			)}
			{!props.hideIcon && props.type === "warning" && (
				<FiAlertCircle size={16} class={`text-warning ${props.truncate ? "shrink-0" : ""}`} />
			)}
			{!props.hideIcon && props.type === "success" && (
				<FiCheckCircle size={16} class={`text-success ${props.truncate ? "shrink-0" : ""}`} />
			)}
			<span class={`text-${props.type} text-sm ${props.truncate ? "truncate leading-4" : ""}`}>
				{props.message}
			</span>
		</span>
	);
};

export default Alert;

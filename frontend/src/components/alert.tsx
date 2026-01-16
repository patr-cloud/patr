import { FiAlertCircle, FiCheckCircle } from "solid-icons/fi";

interface AlertProps {
	/** The alert message to display */
	message: string;
	/** The type of alert */
	type: "error" | "success" | "warning";
	/** Additional Classes to apply */
	class?: string;
}

const Alert = (props: AlertProps) => {
	return (
		<span class={`${props.class ?? ""} text-white flex items-center gap-2 justify-start`}>
			{props.type === "error" && <FiAlertCircle size={16} class="text-error" />}
			{props.type === "warning" && <FiAlertCircle size={16} class="text-warning" />}
			{props.type === "success" && <FiCheckCircle size={16} class="text-success" />}
			<span class={`text-${props.type} text-sm`}>{props.message}</span>
		</span>
	);
};

export default Alert;

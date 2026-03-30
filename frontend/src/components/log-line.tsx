import { parseDate } from "~/utils/func";

interface LogLineProps {
	/** The log entry with timestamp and message */
	log: {
		timestamp: Date | string;
		log: string;
	};
	/** The line number to display in the gutter */
	lineNum: number;
}

/** A single log line with line number, timestamp, and message. Terminal-style. */
const LogLine = (props: LogLineProps) => {
	const ts = () => {
		const d = parseDate(props.log.timestamp);
		if (!d) return "--:--:--";
		return d.toLocaleTimeString([], {
			hour: "2-digit",
			minute: "2-digit",
			second: "2-digit",
			hour12: false,
		});
	};

	return (
		<div class="group flex w-full font-log text-xs leading-6 hover:bg-white/[0.03] transition-colors duration-75 select-text">
			<span class="w-10 shrink-0 text-right pr-sm text-white/15 group-hover:text-white/25 select-none tabular-nums">
				{props.lineNum}
			</span>
			<span class="w-20 shrink-0 text-primary/60 group-hover:text-primary/80 select-none tabular-nums">
				{ts()}
			</span>
			<span class="text-white/70 group-hover:text-white/90 break-all pl-sm">{props.log.log}</span>
		</div>
	);
};

export default LogLine;

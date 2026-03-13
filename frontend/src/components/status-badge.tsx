import { mergeProps } from "solid-js";
import { Color } from "~/utils/color";
import { get, getColorClasses } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface StatusBadgeProps {
	/** Additional Classes for the badge.  */
	class?: MaybeAccessor<string>;
	/** The Text of the status Badge */
	text?: MaybeAccessor<string>;
	/** Status Color */
	color?: Color;
}

const StatusBadge = (rawProps: StatusBadgeProps) => {
	const props = mergeProps(
		{
			class: "",
			color: Color.Info,
		},
		rawProps
	);
	const colorClasses = () => getColorClasses(props.color);
	return (
		<span class={`relative text-secondary cursor-default py-0.25 px-1.5 rounded-xl ${colorClasses().bg}`}>{get(props.text)}</span>
	);
};

export default StatusBadge;

import { mergeProps } from "solid-js";
import { Color } from "~/utils/color";
import { get, getColorClasses } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface InitialsProps {
	/** First name of the user */
	firstName: MaybeAccessor<string | undefined>;
	/** Last name of the user */
	lastName?: MaybeAccessor<string | undefined>;
	/** Size variant */
	size?: "xs" | "sm" | "md" | "lg";
	/** Additional CSS classes */
	class?: MaybeAccessor<string | undefined>;
	/**
	 * Color of the intitials
	 */
	color?: Color;
	/** Background color of the circle. Defaults to `bg-secondary-dark`. */
	bgColor?: Color;
}

const sizeClasses = {
	xs: "w-6 h-6 text-xs",
	sm: "w-8 h-8 text-sm",
	md: "w-10 h-10 text-base",
	lg: "w-12 h-12 text-lg",
} as const;

const Initials = (rawProps: InitialsProps) => {
	const props = mergeProps(
		{
			size: "sm" as const,
			color: Color.White,
		},
		rawProps
	);

	const getInitials = (firstName?: string, lastName?: string): string => {
		if (lastName) {
			return `${firstName?.[0] || ""}${lastName[0]}`.toUpperCase();
		}

		return (firstName?.slice(0, 2) ?? "??").toUpperCase();
	};

	const bgClass = () => (props.bgColor ? getColorClasses(props.bgColor).bg : "bg-secondary-dark");

	return (
		<div
			class={`${bgClass()} rounded-full flex items-center justify-center font-light ${sizeClasses[props.size]} ${get(props.class)} ${getColorClasses(props.color).text}`}
		>
			{getInitials(get(props.firstName), get(props.lastName))}
		</div>
	);
};

export default Initials;

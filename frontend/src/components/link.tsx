import { A } from "@solidjs/router";
import { ParentProps, mergeProps, JSX } from "solid-js";
import { Color, ButtonVariantEnum, ButtonVariant } from "~/utils/color";
import { get, getColorClasses } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface LinkProps {
	/**
	 * The href/path for the link
	 */
	href: string;
	/**
	 * Additional classes for the link
	 */
	class?: MaybeAccessor<string | undefined>;
	/**
	 * Button variant style (only applies when variant="button")
	 */
	buttonVariant?: ButtonVariantEnum;
	/**
	 * Whether it's an external link (uses <a> instead of <A>)
	 */
	external?: boolean;
	/**
	 * The color of the link, defaults to Color.Primary.
	 */
	color?: Color;
	/**
	 * The target attribute specifies where to open the linked document. Only applies when external is true.
	 */
	target?: "_self" | "_blank" | "_parent" | "_top";
}

const Link = (rawProps: ParentProps<LinkProps>) => {
	const props = mergeProps(
		{
			class: "",
			variant: "link" as const,
			buttonVariant: ButtonVariant.Plain,
			external: false,
			color: Color.Primary,
		},
		rawProps
	);

	let derivedClass = () => {
		const variant = () => {
			const colors = getColorClasses(props.color);
			switch (props.buttonVariant) {
				case ButtonVariant.Outlined:
					return `border-2 font-normal ${colors.border} py-xs px-md ${colors.text} rounded-xs ${colors.hoverBg} hover:text-secondary hover:cursor-pointer transition-all duration-200`;
				case ButtonVariant.Plain:
					return `bg-transparent ${colors.text}`;
				case ButtonVariant.Contained:
					return `${colors.bg} text-secondary py-xs px-md rounded-xs font-thin border-2  ${colors.border} \
						${colors.hoverBorder} hover:cursor-pointer hover:bg-transparent ${colors.hoverText} \
						disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200`;
			}
		};

		return `flex items-center ${variant()} justify-center ${get(props.class) ?? ""}`;
	};

	if (props.external) {
		return (
			<a target={props.target} href={props.href} class={derivedClass()}>
				{props.children}
			</a>
		);
	}

	return (
		<A target={props.target} href={props.href} class={derivedClass()}>
			{props.children}
		</A>
	);
};

export default Link;

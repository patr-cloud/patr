import { A } from "@solidjs/router";
import { ParentProps, mergeProps, JSX } from "solid-js";
import { Color, ButtonVariantEnum, ButtonVariant } from "~/utils/color";
import { get } from "~/utils/func";
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
}

const Link = (rawProps: ParentProps<LinkProps>) => {
	const props = mergeProps(
		{
			class: "",
			variant: "link" as const,
			buttonVariant: ButtonVariant.Plain,
			external: false,
		},
		rawProps
	);

	const variant = () => {
		switch (props.buttonVariant) {
			case ButtonVariant.Outlined:
				return "border-2 font-medium border-primary py-xs px-md text-primary rounded-xs";
			case ButtonVariant.Plain:
				return "bg-transparent text-primary";
			case ButtonVariant.Contained:
				return `bg-primary text-secondary py-xs px-md rounded-xs font-thin border-2 border-primary \
						hover:border-primary hover:cursor-pointer hover:bg-transparent hover:text-primary \
						disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200`;
		}
	};

	const derivedClass = () => {
		return `flex items-center ${variant()} justify-center ${get(props.class) ?? ""}`;
	};

	if (props.external) {
		return (
			<a href={props.href} class={derivedClass()}>
				{props.children}
			</a>
		);
	}

	return (
		<A href={props.href} class={derivedClass()}>
			{props.children}
		</A>
	);
};

export default Link;

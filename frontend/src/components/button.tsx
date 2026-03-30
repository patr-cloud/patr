import { JSX, ParentProps, mergeProps } from "solid-js";
import { Color, ButtonVariantEnum, ButtonVariant } from "~/utils/color";
import { get, getColorClasses } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";
import { LoadingSpinner } from "./loading-spinner";

type ButtonProps = {
	/**
	 * The Type of the button, defaults to 'button'.
	 */
	type?: "button" | "submit" | "reset";
	/**
	 * Additional Classes for the button.
	 */
	class?: MaybeAccessor<string | undefined>;
	/**
	 * The color of the button, defaults to Color.Primary.
	 */
	color?: Color;
	/**
	 * Whether the button is disabled or not
	 */
	disabled?: boolean;
	/**
	 * Button Variant, defaults to ButtonVariant.Plain
	 */
	variant?: ButtonVariantEnum;
	/**
	 * Click handler for the button
	 */
	onClick?: (event: MouseEvent & { currentTarget: HTMLButtonElement }) => void;
	/**
	 * Loading state for the button, if true, shows a loading spinner and disables the button
	 */
	loading?: MaybeAccessor<boolean>;
	/**
	 * Loading Content
	 */
	loadingContent?: () => JSX.Element;
};

const Button = (rawProps: ParentProps<ButtonProps>) => {
	const props = mergeProps(
		{
			disabled: false,
			class: "",
			variant: ButtonVariant.Plain,
			color: Color.Primary,
		},
		rawProps
	);

	let derivedClass = () => {
		const variant = () => {
			const colors = getColorClasses(props.color);

			switch (props.variant) {
				case ButtonVariant.Outlined:
					return `border-2 font-medium ${colors.border} py-xs px-md ${colors.text} rounded-xs ${colors.hoverBg} enabled:hover:text-secondary enabled:hover:cursor-pointer transition-all duration-200`;
				case ButtonVariant.Plain:
					return `bg-transparent ${colors.text}`;
				case ButtonVariant.Contained:
					return `${colors.bg} text-secondary py-xs px-md rounded-xs font-thin border-2  ${colors.border} \
						${colors.hoverBorder} enabled:hover:cursor-pointer enabled:hover:bg-transparent ${colors.hoverText} \
						disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200`;
			}
		};

		return `flex items-center ${variant()} justify-center ${get(props.class) ?? ""}`;
	};

	return (
		<button
			disabled={props.disabled || get(props.loading)}
			type={props.type}
			class={derivedClass()}
			onClick={(e) => props.onClick?.(e)}
		>
			{get(props.loading) && props.loadingContent ? (
				<div class="flex items-center gap-2">
					<LoadingSpinner size={20} />
					{props.loadingContent()}
				</div>
			) : (
				props.children
			)}
		</button>
	);
};

export default Button;

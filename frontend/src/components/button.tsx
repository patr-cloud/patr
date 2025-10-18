import { ParentProps, Accessor, mergeProps } from "solid-js";
import { Color, ButtonVariantEnum, ButtonVariant } from "~/utils/color";
import get from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface ButtonProps {
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
  onClick?: () => void;
}

const Button = (rawProps: ParentProps<ButtonProps>) => {
  const props = mergeProps(
    {
      disabled: false,
      class: "",
      variant: ButtonVariant.Plain,
    },
    rawProps
  );

  let derivedClass = () => {
    const variant = () => {
      switch (props.variant) {
        case ButtonVariant.Outlined:
          return "font-medium ";
        case ButtonVariant.Plain:
          return "bg-transparent";
        case ButtonVariant.Contained:
          return `bg-primary text-secondary py-xs px-md rounded-xs font-thin border-2 border-primary
            hover:border-primary hover:cursor-pointer hover:bg-secondary hover:text-primary
            disabled:opacity-50 disabled:cursor-not-allowed`;
      }
    };

    return `flex items-center ${variant()} justify-center ${
      get(props.class) ?? ""
    }`;
  };

  return (
    <button
      disabled={props.disabled}
      type={props.type}
      class={`${derivedClass()} bg-${props.color}`}
      onClick={props.onClick}
    >
      {props.children}
    </button>
  );
};

export default Button;

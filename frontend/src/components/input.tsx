import { FiEye, FiEyeOff } from "solid-icons/fi";
import { createSignal, mergeProps, JSX } from "solid-js";
import { get, variantBgClass } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

/// The Type of the input
const InputType = {
  /**
   * The default value. A single-line text field. Line-breaks are
   * automatically removed from the input value.
   */
  Text: "text",
  /**
   * A field for editing an email address. Looks like a text input, but has
   * validation parameters and relevant keyboard in supporting browsers and
   * devices with dynamic keyboards.
   */
  Email: "email",
  /**
   * A single-line text field whose value is obscured. Will alert user if
   * site is not secure.
   */
  Password: "password",
  /**
   * A control for entering a number. Displays a spinner and adds default
   * validation. Displays a numeric keypad in some devices with dynamic
   * keypads.
   */
  Number: "number",
  /**
   * A control for entering a number. Displays a numeric keypad in some
   * devices with dynamic keypads.
   */
  Phone: "phone",
  /**
   * A check box allowing single values to be selected/deselected.
   */
  Checkbox: "checkbox",
  /**
   * An input which allows for the uploading of a file. Will be rendered as
   * a button with a file picker dialog.
   */
  File: "file",
  /**
   * Hidden input, doesn't render on the dom, but it's name field
   * will still be accessed by the _Ancestor Form Element_.
   * Can be used to pass the id, or some other request data.
   */
  Hidden: "hidden",
  /**
   * A radio button, allowing a single value to be selected
   * out of multiple choices with the same name value.
   */
  Radio: "radio",
  /**
   * A control for entering a number whose exact value is not important.
   * Displays as a range widget defaulting to the middle value.
   * Used in conjunction min and max to define the range of acceptable values.
   */
  Range: "range",
  /**
   * A single-line text field for entering search strings.
   * Line-breaks are automatically removed from the input value. May include a delete icon in supporting browsers that can be used to clear the field.
   * Displays a search icon instead of enter key on some devices with dynamic keypads.
   */
  Search: "search",
  /**
   * A button that submits the form.
   */
  Submit: "submit",
  /**
   * A control for entering a telephone number.
   * Displays a telephone keypad in some devices with dynamic keypads.
   */
  Tel: "tel",
  /**
   * A field for entering a URL. Looks like a text input, but has validation parameters and relevant keyboard in supporting browsers and devices with dynamic keyboards.
   */
  Url: "url",
  /**
   * A control for specifying a color; opening a color picker when active in supporting browsers.
   */
  Color: "color",
  /**
   * A control for entering a date and time, with no time zone.
   * Opens a date picker or numeric wheels for date- and time-components when active in supporting browsers.
   */
  DatetimeLocal: "datetime-local",
  /**
   * A Calendar like date picker.
   */
  Date: "date",
  /**
   * A control for entering a time value with no time zone.
   */
  Time: "time",
  /**
   * A control for entering a date consisting of a week-year number and a week number with no time zone.
   */
  Week: "week",
  /**
   * A control for entering a month and year, with no time zone.
   */
  Month: "month",
};

export type InputVariantEnum = (typeof InputType)[keyof typeof InputType];
export type InputEventT = InputEvent & { currentTarget: HTMLInputElement };

interface InputProps {
  /** The name of the input, this is used to identify the input in a form submission. */
  name?: string;
  /** The Type of the input, defaults to InputType.Text */
  type?: InputVariantEnum;
  /** The ID of the input, this is used to identify the input in the DOM.  */
  id?: string;
  /** Additional Classes for the input. */
  class?: MaybeAccessor<string>;
  /** Specifies whether the form field needs to be filled in before it can be submitted, doesn't use javascript.  */
  required?: boolean;
  /** The placeholder text for the input */
  placeholder?: string;
  /** The Form Id of the input */
  form?: string;
  /** On Input Handler */
  onInput?: (e: InputEventT) => void;
  /** On Change Handler */
  onChange?: (e: Event) => void;
  /** On KeyDown Handler */
  onKeyDown?: (e: KeyboardEvent & { currentTarget: HTMLInputElement }) => void;
  /** On Paste Handler */
  onPaste?: (e: ClipboardEvent & { currentTarget: HTMLInputElement }) => void;
  /** Whether the input is disabled or not */
  disabled?: MaybeAccessor<boolean>;
  /** Label for the input, if undefined, no label is rendered. */
  label?: MaybeAccessor<string>;
  /** The value of the input, this is used to set the initial value of the input.  */
  value?: MaybeAccessor<string | number | string[] | undefined>;
  /** The Color Variant of the input */
  styleVariant?: "light" | "medium" | "dark";
  /** The End Icon of the input */
  endIcon?: () => JSX.Element;
  /** The Start Icon of the input */
  startIcon?: () => JSX.Element;
  /** The pattern attribute of the input */
  pattern?: string;
  /** Maximum length of the input value */
  maxLength?: number;
  /** Additional classes for the inner input element */
  innerClass?: MaybeAccessor<string>;
  /**
   * Global attribute valid for all elements, including all input types, containing a text representing advisory information related to the element it belongs to.
   * {@link https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/input#title MDN Documentation}
   */

  title?: string;
}

const Input = (rawProps: InputProps) => {
  const props = mergeProps(
    {
      type: InputType.Text,
      class: () => "",
      styleVariant: "light",
    },
    rawProps
  );

  const containerClass = `rounded-xs flex justify-start
    items-center border border-secondary-medium
    transition-all duration-250
    focus-within:border-primary focus-within:shadow-md focus-within:bg-secondary-light
    ${variantBgClass(get(props.styleVariant))} ${get(props.class)} ${
    get(props.disabled) ? "bg-secondary-medium cursor-not-allowed" : ""
  }`;

  const paddingClass = () => {
    const hasStart = props.startIcon;
    const hasEnd = props.endIcon;

    if (hasStart && hasEnd) return "py-xs px-xs";
    if (hasStart) return "py-xs pl-xs pr-md";
    if (hasEnd) return "py-xs pl-md pr-xs";
    return "py-xs px-lg";
  };

  return (
    <div class={containerClass}>
      {props.label && <label>{get(props.label)}</label>}
      {props.startIcon && <>{props.startIcon()}</>}
      <input
        title={props.title}
        form={props.form}
        required={props.required}
        class={`overflow-hidden text-ellipsis w-full text-white border-none bg-transparent disabled:text-disabled focus:outline-none placeholder:text-grey text-sm font-thin ${paddingClass()} ${get(
          props.innerClass
        )}`}
        pattern={props.pattern}
        onInput={props.onInput}
        onChange={props.onChange}
        onKeyDown={props.onKeyDown}
        onPaste={props.onPaste}
        placeholder={props.placeholder}
        disabled={get(props.disabled)}
        id={props.id}
        name={props.name}
        value={get(props.value) ?? ""}
        type={props.type}
        maxLength={props.maxLength}
      />
      <div class="pr-5 flex items-center justify-center">
        {props.endIcon && <div>{props.endIcon()}</div>}
      </div>
    </div>
  );
};

export const PasswordInput = (props: InputProps) => {
  const [showPassword, setShowPassword] = createSignal<boolean>(false);

  return (
    <Input
      {...props}
      type={showPassword() ? InputType.Text : InputType.Password}
      endIcon={() => (
        <button
          type="button"
          onClick={() => setShowPassword(!showPassword())}
          class="text-primary"
        >
          {showPassword() ? <FiEye /> : <FiEyeOff />}
        </button>
      )}
    />
  );
};
export { InputType };
export default Input;

import { mergeProps } from "solid-js";
import { JSX } from "solid-js/h/jsx-runtime";
import get from "~/utils/func";
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

interface InputProps {
  /**
   * The name of the input, this is used to identify the input in a form submission.
   */
  name?: string;
  /**
   * The Type of the input, defaults to InputType.Text
   */
  type?: InputVariantEnum;
  /**
   * The ID of the input, this is used to identify the input in the DOM.
   */
  id?: string;
  /**
   * Additional Classes for the input.
   */
  class?: MaybeAccessor<string>;
  /**
   * Specifies whether the form field needs to be filled in before it can
   * be submitted, doesn't use javascript.
   */
  required?: boolean;
  /**
   * The placeholder text for the input
   */
  placeholder?: string;
  /**
   * The Form Id of the input
   */
  form?: string;
  /**
   * On Input Handler
   */
  onInput?: (e: Event) => void;
  /**
   * On Change Handler
   */
  onChange?: (e: Event) => void;
  /**
   * Whether the input is disabled or not
   */
  disabled?: boolean;
  /**
   * Label for the input, if undefined, no label is rendered.
   */
  label?: MaybeAccessor<string>;
  /**
   * The value of the input, this is used to set the initial value of the input.
   */
  value?: MaybeAccessor<string | number | string[] | undefined>;
  /**
   * The Color Variant of the input
   */
  styleVariant?: "light" | "medium" | "dark";
  /**
   * The End Icon of the input
   */
  endIcon?: JSX.Element;
  /**
   * End Text of the input
   */
  endText?: string;
  /**
   * The Start Icon of the input
   */
  startIcon?: JSX.Element;
  /**
   * Start Text of the input
   */
  startText?: string;
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
    items-center border border-secondary-medium px-sm
    transition-all duration-250
    focus-within:border-primary focus-within:shadow-md
    bg-secondary-light ${get(props.class)}`;

  const paddingClass = () => {
    const hasStart = props.startIcon || props.startText;
    const hasEnd = props.endIcon || props.endText;

    if (hasStart && hasEnd) return "py-xs px-xs";
    if (hasStart) return "py-xs pl-xs pr-md";
    if (hasEnd) return "py-xs pl-md pr-xs";
    return "py-xs px-lg";
  };

  return (
    <div class={containerClass}>
      {props.label && <label>{get(props.label)}</label>}
      {props.startText && (
        <span class="text-gray-400 text-sm pl-md">{props.startText}</span>
      )}
      {props.startIcon && <>{props.startIcon}</>}
      <input
        class={`overflow-hidden text-sm text-ellipsis w-full text-white font-thin border-none bg-transparent disabled:text-disabled focus:outline-none placeholder:text-grey ${paddingClass()}`}
        onInput={props.onInput}
        onChange={props.onChange}
        placeholder={props.placeholder}
        disabled={props.disabled}
        id={props.id}
        name={props.name}
        value={get(props.value) ?? ""}
        type={props.type}
      />
      {props.endText && <span>{props.endText}</span>}
      {props.endIcon && <>{props.endIcon}</>}
    </div>
  );
};

export { InputType };
export default Input;

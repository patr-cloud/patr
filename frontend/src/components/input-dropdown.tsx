import { createSignal, JSX } from "solid-js";
import { useClickOutside } from "~/hooks";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

export interface InputDropdownOption {
  /** The Label of to be rendered */
  label: string;
  /** The Value of the option, e.g. id, index etc */
  value: string;
}

interface InputDropdownProps {
  /** Dropdown Options */
  options: MaybeAccessor<InputDropdownOption[]>;
  /** On Select Option */
  onSelect: (value: string) => void;
  /** Additional Classes for the input.  */
  class?: MaybeAccessor<string>;
  /** The placeholder text for the input */
  placeholder?: string;
  /** The currently selected value */
  value?: MaybeAccessor<string | undefined>;
  /** The Color Variant of the input */
  styleVariant?: "light" | "medium" | "dark";
  /** Whether the input is disabled or not */
  disabled?: MaybeAccessor<boolean>;
  /**
   * Specifies whether the form field needs to be filled in before it can
   * be submitted, doesn't use javascript.
   */
  required?: boolean;
  /** The ID of the input, this is used to identify the input in the DOM. */
  id?: string;
  /** The name of the input, this is used to identify the input in a form submission. */
  name?: string;
  /** End Icon */
  endIcon?: () => JSX.Element;
  /** On Click End Icon */
  onClickEndIcon?: () => void;
}

const InputDropdown = (props: InputDropdownProps) => {
  const containerClass = `rounded-xs flex justify-start
    items-center border border-secondary-medium relative
    transition-all duration-250
    focus-within:border-primary focus-within:shadow-md
    bg-secondary-light ${get(props.class)}`;

  const [showDropdown, setShowDropdown] = createSignal(false);
  //   let ref: HTMLDivElement | undefined;
  const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
  useClickOutside(dropdownRef, () => {
    console.log("Clicked outside");
    setShowDropdown(false);
  });

  const onSelectItem = (e: MouseEvent, value: string) => {
    e.stopPropagation();
    console.log("Selected", value);
    props.onSelect(value);
    setShowDropdown(false);
  };

  const dropdownValue = () => {
    const dropdownOptions = get(props.options);
    const selectedValue = get(props.value);

    return (
      dropdownOptions.find((opt) => opt.value === selectedValue)?.label || ""
    );
  };

  return (
    <div
      ref={setDropdownRef}
      onClick={() => setShowDropdown((prev) => !prev)}
      class={containerClass}
    >
      <input
        required={props.required}
        value={dropdownValue()}
        id={props.id}
        name={props.name}
        disabled={get(props.disabled)}
        class={`mx-sm overflow-hidden text-sm text-ellipsis w-full text-white font-thin border-none bg-transparent disabled:text-disabled focus:outline-none placeholder:text-grey py-xs px-lg`}
        type="text"
        placeholder={props.placeholder}
      />

      {showDropdown() && (
        <div class="bg-secondary-medium absolute z-10 top-12 left-0 w-full rounded-xs shadow-lg">
          {get(props.options).map((option) => (
            <div
              onClick={(e) => onSelectItem(e, option.value)}
              class="border-b last-of-type:border-0 border-grey px-xl py-sm cursor-pointer"
            >
              {option.label}
            </div>
          ))}
          {get(props.options).length === 0 && (
            <div class="px-xl py-sm text-grey">No options available.</div>
          )}
        </div>
      )}
    </div>
  );
};

export default InputDropdown;

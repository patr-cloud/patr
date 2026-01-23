import { FiChevronDown } from "solid-icons/fi";
import { createSignal, For, JSX, mergeProps } from "solid-js";
import { useClickOutside } from "~/hooks";
import { get, variantBgClass } from "~/utils/func";
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

const InputDropdown = (rawProps: InputDropdownProps) => {
	const props = mergeProps(
		{
			class: () => "",
			styleVariant: "light",
		},
		rawProps
	);

	const [showDropdown, setShowDropdown] = createSignal(false);
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	const [inputRef, setInputRef] = createSignal<HTMLInputElement>();
	const [inputValue, setInputValue] = createSignal("");
	const [highlightedIndex, setHighlightedIndex] = createSignal(-1);

	useClickOutside(dropdownRef, () => {
		setShowDropdown(false);
		setHighlightedIndex(-1);
	});

	const onSelectItem = (e: MouseEvent, value: string) => {
		e.stopPropagation();
		props.onSelect(value);
		setShowDropdown(false);
		setInputValue(""); // Reset filter after selection
	};

	const containerClass = () => `rounded-xs flex justify-start
		items-center border border-secondary-medium relative
		transition-all duration-250
		focus-within:border-primary focus-within:shadow-md focus-within:bg-secondary-light ${
			showDropdown() ? "rounded-b-none" : ""
		}
		${variantBgClass(get(props.styleVariant))} ${get(props.class)}
	`;

	const dropdownValue = () => {
		const dropdownOptions = get(props.options);
		const selectedValue = get(props.value);

		return dropdownOptions.find((opt) => opt.value === selectedValue)?.label || "";
	};

	const filteredOptions = () => {
		const options = get(props.options);
		const filter = inputValue().toLowerCase();

		if (!filter) {
			return options;
		}

		return options.filter(
			(option) => option.label.toLowerCase().includes(filter) || option.value.toLowerCase().includes(filter)
		);
	};

	const onInputChange = (e: Event) => {
		const target = e.target as HTMLInputElement;
		setInputValue(target.value);
		setShowDropdown(true);
		setHighlightedIndex(-1);
	};

	const onKeyDown = (e: KeyboardEvent) => {
		const options = filteredOptions();

		if (!showDropdown() && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
			e.preventDefault();
			setShowDropdown(true);
			setHighlightedIndex(0);
			return;
		}

		if (!showDropdown()) return;

		switch (e.key) {
			case "ArrowDown":
				e.preventDefault();
				setHighlightedIndex((prev) => (prev < options.length - 1 ? prev + 1 : prev));
				break;
			case "ArrowUp":
				e.preventDefault();
				setHighlightedIndex((prev) => (prev > 0 ? prev - 1 : 0));
				break;
			case "Enter":
				e.preventDefault();
				if (highlightedIndex() >= 0 && highlightedIndex() < options.length) {
					props.onSelect(options[highlightedIndex()].value);
					setShowDropdown(false);
					setInputValue("");
					setHighlightedIndex(-1);
				}
				break;
			case "Escape":
				e.preventDefault();
				setShowDropdown(false);
				setInputValue("");
				setHighlightedIndex(-1);
				inputRef()?.blur();
				break;
			case "Home":
				e.preventDefault();
				setHighlightedIndex(0);
				break;
			case "End":
				e.preventDefault();
				setHighlightedIndex(options.length - 1);
				break;
		}
	};

	return (
		<div ref={setDropdownRef} onClick={() => setShowDropdown((prev) => !prev)} class={containerClass()}>
			<input
				ref={setInputRef}
				required={props.required}
				value={showDropdown() ? inputValue() : dropdownValue()}
				onInput={onInputChange}
				onKeyDown={onKeyDown}
				id={props.id}
				name={props.name}
				disabled={get(props.disabled)}
				class={`overflow-hidden text-sm text-ellipsis w-full text-white font-thin border-none bg-transparent disabled:text-disabled focus:outline-none placeholder:text-grey py-xs px-lg`}
				type="text"
				placeholder={props.placeholder}
			/>

			<FiChevronDown class="mr-sm" />

			{showDropdown() && (
				<div
					class={`${variantBgClass(
						get(props.styleVariant)
					)} border border-border-color absolute z-10 top-[2.22rem] -left-px w-[calc(100%+2px)] rounded-xs rounded-t-none shadow-lg overflow-y-scroll max-h-60`}
				>
					<For each={filteredOptions()}>
						{(option, index) => (
							<div
								onClick={(e) => onSelectItem(e, option.value)}
								onMouseEnter={() => setHighlightedIndex(index())}
								class={`border-b last-of-type:border-0 border-border-color hover:bg-secondary-dark px-xl py-sm cursor-pointer ${
									highlightedIndex() === index() ? "bg-secondary-dark" : ""
								}`}
							>
								{option.label}
							</div>
						)}
					</For>
					{filteredOptions().length === 0 && <div class="px-xl py-sm text-grey">No options available.</div>}
				</div>
			)}
		</div>
	);
};

export default InputDropdown;

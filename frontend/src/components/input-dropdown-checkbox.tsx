import { MaybeAccessor } from "~/utils/types";
import { InputDropdownOption } from "./input-dropdown";
import { createEffect, createSignal, For, JSX, mergeProps, Show } from "solid-js";
import { get, variantBgClass } from "~/utils/func";
import { FiChevronDown } from "solid-icons/fi";
import { useClickOutside } from "~/hooks";
import Checkbox from "./checkbox";

interface InputDropdownCheckboxProps {
	/** Dropdown Options */
	options: MaybeAccessor<InputDropdownOption[]>;
	/** List of checked checkboxes */
	checked: MaybeAccessor<string[]>;
	/** Callback when a checkbox is toggled */
	onToggle: (value: string) => void;
	/** Additional Classes for the input.  */
	class?: MaybeAccessor<string>;
	/** The placeholder text for the input */
	placeholder?: MaybeAccessor<string>;
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
	/** Text to put in start */
	startText?: MaybeAccessor<string>;
	/** Called when the user scrolls near the bottom of the dropdown */
	onLoadMore?: () => void;
	/** Whether more items are currently being loaded */
	isLoadingMore?: MaybeAccessor<boolean>;
}

const InputDropdownCheckbox = (rawProps: InputDropdownCheckboxProps) => {
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
	const [dropDirection, setDropDirection] = createSignal<"down" | "up">("down");

	const DROPDOWN_MAX_HEIGHT_PX = 240;

	useClickOutside(dropdownRef, () => {
		setShowDropdown(false);
	});

	createEffect(() => {
		if (!showDropdown()) return;
		const el = dropdownRef();
		if (!el || typeof window === "undefined") return;
		const rect = el.getBoundingClientRect();
		const spaceBelow = window.innerHeight - rect.bottom;
		const spaceAbove = rect.top;
		setDropDirection(spaceBelow < DROPDOWN_MAX_HEIGHT_PX && spaceAbove > spaceBelow ? "up" : "down");
	});

	const containerClass = () => `rounded-xs flex justify-start
		items-center border border-secondary-medium relative
		transition-all duration-250
    ${showDropdown() ? "border-primary shadow-md bg-secondary-light" : ""}
		focus-within:border-primary focus-within:shadow-md focus-within:bg-secondary-light
		${variantBgClass(get(props.styleVariant))} ${get(props.class)} ${
			showDropdown() ? (dropDirection() === "up" ? "rounded-t-none" : "rounded-b-none") : ""
		}`;

	const onSelectItem = (e: MouseEvent, value: string) => {
		e.stopPropagation();
		props.onToggle(value);
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
					props.onToggle(options[highlightedIndex()].value);
					setInputValue("");
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
				value={showDropdown() ? inputValue() : ""}
				onInput={onInputChange}
				onKeyDown={onKeyDown}
				id={props.id}
				name={props.name}
				disabled={get(props.disabled)}
				class={`overflow-hidden text-sm text-ellipsis w-full text-white font-thin border-none bg-transparent disabled:text-disabled focus:outline-none placeholder:text-grey py-xs px-lg`}
				type="text"
				placeholder={get(props.placeholder)}
			/>

			<FiChevronDown class="mr-sm" />

			{showDropdown() && (
				<div
					class={`${variantBgClass(
						get(props.styleVariant)
					)} border border-border-color absolute z-10 -left-px w-[calc(100%+2px)] rounded-xs shadow-lg overflow-y-scroll max-h-60 ${
						dropDirection() === "up" ? "bottom-[2.22rem] rounded-b-none" : "top-[2.22rem] rounded-t-none"
					}`}
					onScroll={(e) => {
						if (!props.onLoadMore) return;
						const el = e.currentTarget;
						if (el.scrollHeight - el.scrollTop - el.clientHeight < 40) {
							props.onLoadMore();
						}
					}}
				>
					<For each={filteredOptions()}>
						{(option, index) => (
							<div
								onClick={(e) => onSelectItem(e, option.value)}
								class={`border-b last-of-type:border-0 border-border-color px-xl py-sm cursor-pointer flex items-center gap-3 ${
									highlightedIndex() === index() ? "bg-secondary-dark" : ""
								}`}
							>
								<div class="pointer-events-none">
									<Checkbox
										checked={get(props.checked).includes(option.value)}
										label={option.label}
									/>
								</div>
							</div>
						)}
					</For>
					{filteredOptions().length === 0 && <div class="px-xl py-sm text-grey">No options available.</div>}
					<Show when={get(props.isLoadingMore)}>
						<div class="flex items-center justify-center py-sm">
							<div class="w-4 h-4 border-2 border-primary/30 border-t-primary rounded-full animate-spin" />
						</div>
					</Show>
				</div>
			)}
		</div>
	);
};

export default InputDropdownCheckbox;

import { FiChevronDown, FiEye, FiEyeOff } from "solid-icons/fi";
import { createEffect, createSignal, For, mergeProps, onCleanup, Show, JSX } from "solid-js";
import { Portal } from "solid-js/web";
import { useClickOutside } from "~/hooks";
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

export interface AutocompleteSuggestion {
	label: string;
	value: string;
}

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
	onChange?: (e: Event & { currentTarget: HTMLInputElement }) => void;
	/** On KeyDown Handler */
	onKeyDown?: (e: KeyboardEvent & { currentTarget: HTMLInputElement }) => void;
	/** On Blur Handler */
	onBlur?: (e: FocusEvent & { currentTarget: HTMLInputElement }) => void;
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
	/**
	 * The autocomplete hint for the browser.
	 * {@link https://developer.mozilla.org/en-US/docs/Web/HTML/Attributes/autocomplete MDN Documentation}
	 */
	autocomplete?:
		| "off"
		| "on"
		| "username"
		| "email"
		| "current-password"
		| "new-password"
		| "one-time-code"
		| "name"
		| "given-name"
		| "family-name"
		| "tel"
		| "url"
		| "organization";
	/**
	 * Autocomplete suggestions. When provided, shows a filterable dropdown of options.
	 * Each suggestion has a `label` (displayed) and `value` (committed on selection).
	 */
	suggestions?: MaybeAccessor<AutocompleteSuggestion[]>;
	/**
	 * Whether the user can type a value not present in `suggestions`.
	 * Defaults to `true`. When `false`, the input behaves like a searchable select —
	 * the user must pick from the list and typing only filters the dropdown.
	 */
	allowCustomValue?: boolean;
	/**
	 * Called when the user selects a suggestion from the dropdown.
	 * Receives the suggestion's `value` (not label).
	 */
	onSelect?: (value: string) => void;
}

const Input = (rawProps: InputProps) => {
	const props = mergeProps(
		{
			type: InputType.Text,
			class: () => "",
			styleVariant: "light",
			allowCustomValue: true,
		},
		rawProps
	);

	// Autocomplete state — only used when suggestions are provided
	const [showDropdown, setShowDropdown] = createSignal(false);
	const [inputText, setInputText] = createSignal("");
	const [highlightedIndex, setHighlightedIndex] = createSignal(-1);
	const [containerRef, setContainerRef] = createSignal<HTMLDivElement>();
	const [dropdownRef, setDropdownRef] = createSignal<HTMLDivElement>();
	const [dropdownRect, setDropdownRect] = createSignal<{
		top: number;
		left: number;
		width: number;
		bottomOffset: number;
		direction: "down" | "up";
	}>({
		top: 0,
		left: 0,
		width: 0,
		bottomOffset: 0,
		direction: "down",
	});

	const DROPDOWN_MAX_HEIGHT_PX = 240;

	const updateDropdownRect = () => {
		const el = containerRef();
		if (!el || typeof window === "undefined") return;
		const rect = el.getBoundingClientRect();
		const spaceBelow = window.innerHeight - rect.bottom;
		const spaceAbove = rect.top;
		const direction = spaceBelow < DROPDOWN_MAX_HEIGHT_PX && spaceAbove > spaceBelow ? "up" : "down";
		setDropdownRect({
			top: rect.bottom,
			left: rect.left,
			width: rect.width,
			bottomOffset: window.innerHeight - rect.top,
			direction,
		});
	};

	createEffect(() => {
		if (!showDropdown()) return;
		updateDropdownRect();
		const onResizeOrScroll = () => updateDropdownRect();
		window.addEventListener("scroll", onResizeOrScroll, true);
		window.addEventListener("resize", onResizeOrScroll);
		onCleanup(() => {
			window.removeEventListener("scroll", onResizeOrScroll, true);
			window.removeEventListener("resize", onResizeOrScroll);
		});
	});

	useClickOutside(containerRef, (event) => {
		const dd = dropdownRef();
		if (dd && dd.contains(event.target as Node)) return;
		setShowDropdown(false);
		setHighlightedIndex(-1);
		if (!props.allowCustomValue) {
			// Reset display text to the committed value's label
			const suggestions = get(props.suggestions) ?? [];
			const committed = get(props.value) as string | undefined;
			setInputText(suggestions.find((s) => s.value === committed)?.label ?? "");
		}
	});

	const hasSuggestions = () => !!get(props.suggestions);

	const filteredSuggestions = () => {
		const suggestions = get(props.suggestions) ?? [];
		const filter = inputText().toLowerCase();
		if (!filter) return suggestions;
		return suggestions.filter(
			(s) => s.label.toLowerCase().includes(filter) || s.value.toLowerCase().includes(filter)
		);
	};

	/** The text to display in the <input> element */
	const displayValue = () => {
		if (!hasSuggestions()) return (get(props.value) as string | number | undefined) ?? "";
		if (showDropdown()) return inputText();
		if (!props.allowCustomValue) {
			// Show the label of the currently committed value
			const suggestions = get(props.suggestions) ?? [];
			const committed = get(props.value) as string | undefined;
			return suggestions.find((s) => s.value === committed)?.label ?? "";
		}
		// allowCustomValue=true: value IS the text
		return (get(props.value) as string | undefined) ?? "";
	};

	const selectSuggestion = (suggestion: AutocompleteSuggestion) => {
		props.onSelect?.(suggestion.value);
		setInputText(suggestion.label);
		setShowDropdown(false);
		setHighlightedIndex(-1);
	};

	const onAutocompleteKeyDown = (e: KeyboardEvent) => {
		const options = filteredSuggestions();

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
					selectSuggestion(options[highlightedIndex()]);
				}
				break;
			case "Escape":
				e.preventDefault();
				setShowDropdown(false);
				setHighlightedIndex(-1);
				if (!props.allowCustomValue) {
					const suggestions = get(props.suggestions) ?? [];
					const committed = get(props.value) as string | undefined;
					setInputText(suggestions.find((s) => s.value === committed)?.label ?? "");
				}
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

	const containerClass = () => `relative rounded-xs flex justify-start
    items-center border border-secondary-medium
    transition-all duration-125
    focus-within:border-primary focus-within:shadow-md focus-within:bg-secondary-light
    ${variantBgClass(get(props.styleVariant))} ${get(props.class)} ${
		get(props.disabled) ? "bg-secondary-primary cursor-not-allowed" : ""
	} ${
		hasSuggestions() && showDropdown()
			? dropdownRect().direction === "up"
				? "rounded-t-none"
				: "rounded-b-none"
			: ""
	}`;

	const paddingClass = () => {
		const hasStart = props.startIcon;
		const hasEnd = props.endIcon || hasSuggestions();

		if (hasStart && hasEnd) return "py-xs px-xs";
		if (hasStart) return "py-xs pl-xs pr-md";
		if (hasEnd) return "py-xs pl-md pr-xs";
		return "py-xs px-lg";
	};

	return (
		<div ref={hasSuggestions() ? setContainerRef : undefined} class={containerClass()}>
			{props.label && <label>{get(props.label)}</label>}
			{props.startIcon && <>{props.startIcon()}</>}
			<input
				title={props.title}
				form={props.form}
				autocomplete={props.autocomplete}
				required={props.required}
				class={`overflow-hidden text-ellipsis w-full text-white border-none bg-transparent disabled:text-disabled focus:outline-none placeholder:text-grey text-sm font-thin ${paddingClass()} ${get(
					props.innerClass
				)}`}
				pattern={props.pattern}
				onInput={(e) => {
					if (hasSuggestions()) {
						setInputText(e.currentTarget.value);
						setShowDropdown(true);
						setHighlightedIndex(-1);
						if (props.allowCustomValue) {
							props.onInput?.(e);
						}
					} else {
						props.onInput?.(e);
					}
				}}
				onChange={(e) => props.onChange?.(e)}
				onKeyDown={(e) => {
					if (hasSuggestions()) {
						onAutocompleteKeyDown(e);
					}
					props.onKeyDown?.(e);
				}}
				onBlur={(e) => props.onBlur?.(e)}
				onFocus={() => {
					if (hasSuggestions()) {
						setInputText("");
						setShowDropdown(true);
					}
				}}
				onClick={() => {
					if (hasSuggestions()) {
						setInputText("");
						setShowDropdown(true);
					}
				}}
				onPaste={(e) => props.onPaste?.(e)}
				placeholder={props.placeholder}
				disabled={get(props.disabled)}
				id={props.id}
				name={props.name}
				{...(props.type !== InputType.File && {
					value: hasSuggestions() ? displayValue() : (get(props.value) ?? ""),
				})}
				type={props.type}
				maxLength={props.maxLength}
			/>
			<Show
				when={hasSuggestions()}
				fallback={
					<div class="pr-lg flex items-center justify-center">
						{props.endIcon && <div>{props.endIcon()}</div>}
					</div>
				}
			>
				<FiChevronDown class="mr-sm shrink-0" />
			</Show>

			<Show when={hasSuggestions() && showDropdown()}>
				<Portal>
					<div
						ref={setDropdownRef}
						style={{
							position: "fixed",
							...(dropdownRect().direction === "up"
								? { bottom: `${dropdownRect().bottomOffset}px` }
								: { top: `${dropdownRect().top}px` }),
							left: `${dropdownRect().left}px`,
							width: `${dropdownRect().width}px`,
						}}
						class={`${variantBgClass(
							get(props.styleVariant)
						)} border border-border-color z-50 rounded-xs shadow-lg overflow-y-scroll max-h-60 ${
							dropdownRect().direction === "up" ? "rounded-b-none" : "rounded-t-none"
						}`}
					>
						<For each={filteredSuggestions()}>
							{(suggestion, index) => (
								<div
									onMouseDown={(e) => {
										e.preventDefault();
										selectSuggestion(suggestion);
									}}
									onMouseEnter={() => setHighlightedIndex(index())}
									class={`border-b last-of-type:border-0 border-border-color hover:bg-secondary-dark px-xl py-sm cursor-pointer text-sm text-white font-thin ${
										highlightedIndex() === index() ? "bg-secondary-dark" : ""
									}`}
								>
									{suggestion.label}
								</div>
							)}
						</For>
						<Show when={filteredSuggestions().length === 0}>
							<div class="px-xl py-sm text-grey text-sm">No options available.</div>
						</Show>
					</div>
				</Portal>
			</Show>
		</div>
	);
};

export const FileInput = (props: InputProps) => {
	return (
		<Input
			innerClass="input-file"
			startIcon={() => (
				<p class="w-3/4 h-full p-xs flex items-center justify-center bg-secondary-medium">Choose File</p>
			)}
			{...props}
			type={InputType.File}
		/>
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
					class="text-primary flex items-center justify-center"
				>
					{showPassword() ? <FiEye /> : <FiEyeOff />}
				</button>
			)}
		/>
	);
};
export { InputType };
export default Input;

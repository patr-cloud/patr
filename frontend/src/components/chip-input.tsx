import { createSignal, For, Show } from "solid-js";
import { FiX } from "solid-icons/fi";

interface ChipInputProps {
	/** Current list of values */
	values: () => string[];
	/** Called when the list changes */
	onChange: (values: string[]) => void;
	/** Validate a single input value. Return error string or undefined if valid. */
	validate?: (value: string) => string | undefined;
	/** Placeholder for the text input */
	placeholder?: string;
	/** Additional class for the container */
	class?: string;
}

const ChipInput = (props: ChipInputProps) => {
	const [inputValue, setInputValue] = createSignal("");
	const [error, setError] = createSignal("");

	const addChip = (raw: string) => {
		const value = raw.trim();
		if (!value) return;

		// Check for duplicates
		if (props.values().includes(value)) {
			setError(`"${value}" is already added.`);
			return;
		}

		// Validate if validator provided
		if (props.validate) {
			const err = props.validate(value);
			if (err) {
				setError(err);
				return;
			}
		}

		setError("");
		props.onChange([...props.values(), value]);
		setInputValue("");
	};

	const removeChip = (index: number) => {
		const newValues = [...props.values()];
		newValues.splice(index, 1);
		props.onChange(newValues);
	};

	const onKeyDown = (e: KeyboardEvent) => {
		if (e.key === "Enter" || e.key === " " || e.key === ",") {
			e.preventDefault();
			addChip(inputValue());
		} else if (e.key === "Backspace" && inputValue() === "" && props.values().length > 0) {
			removeChip(props.values().length - 1);
		}
	};

	const onInput = (e: Event) => {
		const value = (e.currentTarget as HTMLInputElement).value;
		// If user pastes something with commas, split and add each
		if (value.includes(",")) {
			const parts = value.split(",");
			for (const part of parts) {
				const trimmed = part.trim();
				if (trimmed) addChip(trimmed);
			}
			setInputValue("");
		} else {
			setInputValue(value);
			if (error()) setError("");
		}
	};

	let inputRef!: HTMLInputElement;

	return (
		<div class={props.class}>
			<div
				class={`flex flex-wrap items-center gap-1.5 px-lg py-xs bg-secondary-light border rounded-xs transition-all duration-200 cursor-text ${
					error() ? "border-error" : "border-secondary-medium focus-within:border-primary"
				}`}
				onClick={() => inputRef?.focus()}
			>
				<For each={props.values()}>
					{(value, index) => (
						<span class="chip-tag">
							{value}
							<button
								type="button"
								class="flex items-center justify-center w-4 h-4 rounded-sm bg-white/10 hover:bg-white/20 transition-colors"
								aria-label={`Remove ${value}`}
								onClick={(e) => {
									e.stopPropagation();
									removeChip(index());
								}}
							>
								<FiX size={10} color="#9ca3af" />
							</button>
						</span>
					)}
				</For>
				<input
					ref={inputRef}
					type="text"
					class="chip-input-inner"
					placeholder={props.values().length === 0 ? props.placeholder || "Type and press Enter" : ""}
					value={inputValue()}
					onInput={onInput}
					onKeyDown={onKeyDown}
				/>
			</div>
			<Show when={error()}>
				<p class="text-error text-xs mt-1">{error()}</p>
			</Show>
		</div>
	);
};

export default ChipInput;

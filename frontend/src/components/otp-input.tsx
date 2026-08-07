import { For, Show } from "solid-js";
import { InputType, InputVariantEnum } from "./input";
import { MaybeAccessor } from "~/utils/types";
import { get, variantBgClass } from "~/utils/func";

interface OtpInputProps {
	outerClass?: string;
	inputClass?: string;
	inputVariant?: "light" | "medium" | "dark";
	otpDigits: MaybeAccessor<string[]>;
	setOtpDigits: (digits: string[]) => void;
	/** Number of input boxes. Defaults to 6 (numeric OTP). */
	length?: number;
	/** Filter / transform raw input. Defaults to stripping non-digits. */
	sanitize?: (raw: string) => string;
	/** Input type. Defaults to Tel for the numeric OTP case. */
	inputType?: InputVariantEnum;
	/** Render a separator (em-dash) after this index. */
	separatorAt?: number;
	/** Drop the default styling so caller controls the box appearance entirely. */
	unstyled?: boolean;
	/**
	 * Field name. Defaults to `otp`, which together with `autocomplete` is what
	 * lets password managers (Apple Passwords in particular) autofill a
	 * one-time code.
	 */
	name?: string;
	/** Virtual keyboard hint. Defaults to `numeric` for the OTP case. */
	inputMode?: "numeric" | "text";
	/**
	 * Autofill hint. Defaults to `one-time-code`. Pass `off` for codes that
	 * aren't OTPs (e.g. the runner setup code) so managers don't offer to fill
	 * them.
	 */
	autocomplete?: "one-time-code" | "off";
	/**
	 * Prefix for each box's `id`, rendered as `{idPrefix}-{index}`. Defaults to
	 * `otp`. Focus is driven by refs, not ids — these exist so labels can point
	 * at a box and so callers have a stable hook to address them.
	 */
	idPrefix?: string;
}

const OtpInput = (props: OtpInputProps) => {
	const length = () => props.length ?? 6;
	const sanitize = (raw: string) => (props.sanitize ?? ((s) => s.replace(/\D/g, "")))(raw);

	// Refs to each input element. Indexed by box position so the keyboard
	// handlers can shift focus without resorting to `document.getElementById`.
	const inputs: HTMLInputElement[] = [];
	const focus = (index: number) => inputs[index]?.focus();

	const handleOtpInput = (index: number, value: string) => {
		const ch = sanitize(value).slice(-1);
		const newDigits = [...get(props.otpDigits)];
		newDigits[index] = ch;
		props.setOtpDigits(newDigits);
		if (ch && index < length() - 1) {
			focus(index + 1);
		}
	};

	const handleOtpKeyDown = (index: number, e: KeyboardEvent) => {
		const otpDigits = get(props.otpDigits);

		// Handle backspace - clear current and move to previous
		if (e.key === "Backspace" || e.key === "Delete") {
			if (otpDigits[index]) {
				// Clear current digit and move to previous
				const newDigits = [...otpDigits];
				newDigits[index] = "";
				props.setOtpDigits(newDigits);
				if (index > 0) focus(index - 1);
				e.preventDefault();
			} else if (index > 0) {
				focus(index - 1);
			}
		}
		// Handle arrow keys
		if (e.key === "ArrowLeft" && index > 0) focus(index - 1);
		if (e.key === "ArrowRight" && index < length() - 1) focus(index + 1);
	};

	const handleOtpPaste = (e: ClipboardEvent) => {
		e.preventDefault();
		const otpDigits = get(props.otpDigits);

		const pastedData = e.clipboardData?.getData("text") || "";
		const chars = sanitize(pastedData).slice(0, length()).split("");

		const newDigits = [...otpDigits];
		// Reset the array to the new length so a partial paste doesn't leave stale chars past the end.
		for (let i = 0; i < length(); i++) {
			newDigits[i] = chars[i] ?? "";
		}
		props.setOtpDigits(newDigits);

		// Focus the next empty input or last input
		const nextEmpty = newDigits.findIndex((d) => !d);
		focus(nextEmpty === -1 ? length() - 1 : nextEmpty);
	};

	const defaultBoxClass = () => `w-full text-center text-xl font-medium flex-1 border-none \
		focus:outline focus:outline-solid focus:outline-primary transition-none ${
			props.inputVariant ? variantBgClass(props.inputVariant) : "bg-secondary-light"
		}`;

	return (
		<div class={`flex gap-3 ${props.outerClass ?? ""}`}>
			<For each={Array.from({ length: length() })}>
				{(_, index) => (
					<>
						<input
							ref={(el) => (inputs[index()] = el)}
							id={`${props.idPrefix ?? "otp"}-${index()}`}
							name={props.name ?? "otp"}
							type={props.inputType ?? InputType.Tel}
							inputMode={props.inputMode ?? "numeric"}
							autocomplete={props.autocomplete ?? "one-time-code"}
							maxLength={1}
							value={get(props.otpDigits).at(index()) ?? ""}
							onInput={(e) => handleOtpInput(index(), e.currentTarget.value)}
							onKeyDown={(e) => handleOtpKeyDown(index(), e)}
							onPaste={handleOtpPaste}
							class={`${props.inputClass ?? ""} ${props.unstyled ? "" : defaultBoxClass()}`}
						/>
						<Show when={props.separatorAt !== undefined && index() === props.separatorAt}>
							<span class="font-log text-grey/40 select-none self-center">&mdash;</span>
						</Show>
					</>
				)}
			</For>
		</div>
	);
};

export default OtpInput;

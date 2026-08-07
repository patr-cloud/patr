import { createMemo, createSignal, Show } from "solid-js";
import { Alert, Button, ButtonVariant, InputType, OtpInput } from "~/components";

const USER_CODE_LEN = 8;
const USER_CODE_REGEX = /^[A-HJ-NP-Z2-9]{8}$/;
const sanitizeUserCode = (raw: string) => raw.toUpperCase().replace(/[^A-HJ-NP-Z2-9]/g, "");

export const CodeEntry = (props: { onSubmit: (code: string) => void }) => {
	const [chars, setChars] = createSignal<string[]>(Array(USER_CODE_LEN).fill(""));
	const [error, setError] = createSignal("");
	const code = createMemo(() => chars().join(""));
	const filled = createMemo(() => chars().filter(Boolean).length);

	const submit = (e: Event) => {
		e.preventDefault();
		const c = code();
		if (!USER_CODE_REGEX.test(c)) {
			setError("All 8 boxes must be filled.");
			return;
		}
		props.onSubmit(c);
	};

	return (
		<form
			noValidate
			onSubmit={submit}
			class="mx-auto flex flex-col gap-6"
			style={{ width: "100%", "max-width": "36rem" }}
		>
			<section class="flex flex-col gap-2">
				<h2 class="text-white text-lg font-semibold">Enter your setup code</h2>
				<p class="text-grey text-sm leading-relaxed">
					Run <code class="font-log text-white">patr runner setup</code> on the machine you want to use as a
					runner. The CLI prints an 8-character code - enter it here to load the consent details.
				</p>
			</section>

			{/* Mock terminal output so a user landing here without a code knows what to look for. */}
			<div class="border border-border-color rounded-xs bg-secondary-light/40 overflow-hidden">
				<div class="flex items-center justify-between px-4 py-2 border-b border-border-color/60">
					<span class="text-xxs uppercase tracking-wider text-grey/70">your terminal</span>
					<span class="text-xxs font-log text-grey/50">patr-cli</span>
				</div>
				<div class="px-4 py-3 font-log text-sm leading-relaxed">
					<div class="text-grey/80">
						<span class="text-primary">$</span> patr runner setup
					</div>
					<div class="text-grey/50">If the browser doesn&rsquo;t open, visit:</div>
					<div class="text-white pl-2">https://app.patr.cloud/runner/setup</div>
					<div class="text-grey/50">and enter:</div>
					<div class="text-primary text-base pl-2">ABCD&middot;EFGH</div>
				</div>
			</div>

			<div class="flex flex-col gap-3">
				<OtpInput
					outerClass="justify-center"
					inputClass="w-12 h-12 text-center font-log text-lg uppercase px-0 py-0"
					otpDigits={chars()}
					setOtpDigits={(d) => {
						setChars(d);
						setError("");
					}}
					length={USER_CODE_LEN}
					sanitize={sanitizeUserCode}
					inputType={InputType.Text}
					name="runner-setup-code"
					idPrefix="runner-setup-code"
					inputMode="text"
					autocomplete="off"
					separatorAt={3}
					unstyled
				/>
				<div class="flex items-center justify-between text-xxs text-grey/60 px-1">
					<span>Letters and digits, no confusable characters (no 0, O, 1, I, L)</span>
					<span class="font-log">
						{filled()}/{USER_CODE_LEN}
					</span>
				</div>
				<Show when={error()}>
					<Alert type="error" message={error()} />
				</Show>
			</div>

			<div class="flex justify-center">
				<Button variant={ButtonVariant.Contained} type="submit">
					Continue
				</Button>
			</div>
		</form>
	);
};

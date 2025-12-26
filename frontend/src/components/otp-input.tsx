import { For } from "solid-js";
import Input, { InputType } from "./input";
import { MaybeAccessor } from "~/utils/types";
import { get } from "~/utils/func";

interface OtpInputProps {
  outerClass?: string;
  inputClass?: string;
  inputVariant?: "light" | "medium" | "dark";
  otpDigits: MaybeAccessor<string[]>;
  setOtpDigits: (digits: string[]) => void;
}

const OtpInput = (props: OtpInputProps) => {
  const handleOtpInput = (index: number, value: string) => {
    // Strip non-digits and get last digit
    const digitsOnly = value.replace(/\D/g, "");
    const digit = digitsOnly.slice(-1);

    const newDigits = [...get(props.otpDigits)];
    newDigits[index] = digit;
    props.setOtpDigits(newDigits);
    // Auto-focus next input
    if (digit && index < 5) {
      const nextInput = document.getElementById(`otp-${index + 1}`);
      nextInput?.focus();
    }
  };

  const handleOtpKeyDown = (index: number, e: KeyboardEvent) => {
    const otpDigits = get(props.otpDigits);

    // Handle backspace - clear current and move to previous
    if (e.key === "Backspace") {
      if (otpDigits[index]) {
        // Clear current digit and move to previous
        const newDigits = [...otpDigits];
        newDigits[index] = "";
        props.setOtpDigits(newDigits);
        if (index > 0) {
          const prevInput = document.getElementById(`otp-${index - 1}`);
          prevInput?.focus();
        }
        e.preventDefault();
      } else if (index > 0) {
        // Already empty, just move to previous
        const prevInput = document.getElementById(`otp-${index - 1}`);
        prevInput?.focus();
      }
    }
    // Handle arrow keys
    if (e.key === "ArrowLeft" && index > 0) {
      const prevInput = document.getElementById(`otp-${index - 1}`);
      prevInput?.focus();
    }
    if (e.key === "ArrowRight" && index < 5) {
      const nextInput = document.getElementById(`otp-${index + 1}`);
      nextInput?.focus();
    }
  };

  const handleOtpPaste = (e: ClipboardEvent) => {
    e.preventDefault();
    const otpDigits = get(props.otpDigits);

    const pastedData = e.clipboardData?.getData("text") || "";
    const digits = pastedData.replace(/\D/g, "").slice(0, 6).split("");

    const newDigits = [...otpDigits];
    digits.forEach((digit, i) => {
      newDigits[i] = digit;
    });
    props.setOtpDigits(newDigits);

    // Focus the next empty input or last input
    const nextEmptyIndex = newDigits.findIndex((d) => !d);
    const focusIndex = nextEmptyIndex === -1 ? 5 : nextEmptyIndex;
    document.getElementById(`otp-${focusIndex}`)?.focus();
  };

  return (
    <div class={`${props.outerClass ?? "flex gap-3"}`}>
      <For each={[0, 1, 2, 3, 4, 5]}>
        {(index) => (
          <Input
            styleVariant={props.inputVariant}
            id={`otp-${index}`}
            type={InputType.Tel}
            maxLength={1}
            value={() => get(props.otpDigits)[index]}
            onInput={(e) => handleOtpInput(index, e.currentTarget.value)}
            onKeyDown={(e) => handleOtpKeyDown(index, e)}
            onPaste={handleOtpPaste}
            class={props.inputClass ?? "flex-1"}
            innerClass="text-center text-xl font-medium"
          />
        )}
      </For>
    </div>
  );
};

export default OtpInput;

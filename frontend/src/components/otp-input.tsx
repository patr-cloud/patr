import { createSignal, For, mergeProps, onMount } from "solid-js";

interface OTPInputProps {
  /**
   * Number of OTP digits (default: 6)
   */
  length?: number;
  /**
   * Callback when OTP value changes
   */
  onValueChange?: (value: string) => void;
  /**
   * Callback when OTP is complete
   */
  onComplete?: (value: string) => void;
  /**
   * Whether the input is disabled
   */
  disabled?: boolean;
  /**
   * Whether to show error state
   */
  hasError?: boolean;
  /**
   * Additional CSS classes
   */
  class?: string;
}

const OTPInput = (rawProps: OTPInputProps) => {
  const props = mergeProps(
    {
      length: 6,
      disabled: false,
      hasError: false,
      class: "",
    },
    rawProps
  );

  const [values, setValues] = createSignal<string[]>(Array(props.length).fill(""));
  let inputRefs: HTMLInputElement[] = [];

  const handleInputChange = (index: number, value: string) => {
    // Only allow single digits
    const digit = value.replace(/\D/g, "").slice(-1);
    
    const newValues = [...values()];
    newValues[index] = digit;
    setValues(newValues);

    const otpValue = newValues.join("");
    props.onValueChange?.(otpValue);

    // Auto-advance to next input if digit entered
    if (digit && index < props.length - 1) {
      inputRefs[index + 1]?.focus();
    }

    // Call onComplete if all digits are filled
    if (otpValue.length === props.length) {
      props.onComplete?.(otpValue);
    }
  };

  const handleKeyDown = (index: number, event: KeyboardEvent) => {
    // Handle backspace
    if (event.key === "Backspace") {
      const newValues = [...values()];
      
      if (newValues[index]) {
        // Clear current input
        newValues[index] = "";
        setValues(newValues);
        props.onValueChange?.(newValues.join(""));
      } else if (index > 0) {
        // Move to previous input and clear it
        newValues[index - 1] = "";
        setValues(newValues);
        props.onValueChange?.(newValues.join(""));
        inputRefs[index - 1]?.focus();
      }
    }
    // Handle arrow keys
    else if (event.key === "ArrowLeft" && index > 0) {
      inputRefs[index - 1]?.focus();
    } else if (event.key === "ArrowRight" && index < props.length - 1) {
      inputRefs[index + 1]?.focus();
    }
  };

  const handlePaste = (event: ClipboardEvent) => {
    event.preventDefault();
    const pastedData = event.clipboardData?.getData("text") || "";
    const digits = pastedData.replace(/\D/g, "").slice(0, props.length);
    
    if (digits) {
      const newValues = Array(props.length).fill("");
      for (let i = 0; i < digits.length; i++) {
        newValues[i] = digits[i];
      }
      setValues(newValues);
      props.onValueChange?.(newValues.join(""));
      
      // Focus the next empty input or the last input
      const nextIndex = Math.min(digits.length, props.length - 1);
      inputRefs[nextIndex]?.focus();
      
      // Call onComplete if all digits are filled
      if (digits.length === props.length) {
        props.onComplete?.(digits);
      }
    }
  };

  const getInputClasses = () => {
    const baseClasses = "w-12 h-12 text-center text-lg font-medium rounded-xs border-2 bg-secondary-light text-white focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-secondary transition-all duration-200";
    
    if (props.hasError) {
      return `${baseClasses} border-error focus:border-error focus:ring-error`;
    }
    
    return `${baseClasses} border-secondary-medium focus:border-primary`;
  };

  onMount(() => {
    // Focus first input on mount
    inputRefs[0]?.focus();
  });

  return (
    <div class={`flex gap-sm justify-center ${props.class}`}>
      <For each={Array(props.length).fill(0)}>
        {(_, index) => (
          <input
            ref={(el) => (inputRefs[index()] = el)}
            type="text"
            inputmode="numeric"
            maxlength="1"
            value={values()[index()]}
            disabled={props.disabled}
            class={getInputClasses()}
            onInput={(e) => handleInputChange(index(), e.currentTarget.value)}
            onKeyDown={(e) => handleKeyDown(index(), e)}
            onPaste={handlePaste}
            aria-label={`OTP digit ${index() + 1}`}
          />
        )}
      </For>
    </div>
  );
};

export default OTPInput;
import { A, useNavigate, useSearchParams } from "@solidjs/router";
import { createSignal, For, onMount, Show } from "solid-js";
import { CompleteSignUpRequest } from "~/bindings";
import {
  Button,
  ButtonVariant,
  Input,
  InputType,
  useToast,
  Turnstile,
} from "~/components";

const ConfirmSignUp = () => {
  const navigate = useNavigate();
  const toast = useToast();
  const [searchParams] = useSearchParams<{ username?: string; otp?: string }>();
  const [turnstileToken, setTurnstileToken] = createSignal<string>("");

  // Get username from URL params or navigation state
  const initialUsername = searchParams.username || "";
  const initialOtp = searchParams.otp || "";

  // Track if username was pre-filled (from URL or navigation state)
  const usernameWasPreFilled = !!initialUsername;

  const [username, setUsername] = createSignal(initialUsername);
  const [otpDigits, setOtpDigits] = createSignal<string[]>(
    initialOtp
      ? [...initialOtp.replace(/\D/g, "").slice(0, 6).padEnd(6, " ")].map((c) =>
          c === " " ? "" : c
        )
      : ["", "", "", "", "", ""]
  );

  // Clear URL params after reading them
  onMount(() => {
    if (searchParams.username || searchParams.otp) {
      navigate("/confirm-signup", { replace: true });
    }
  });

  const handleOtpInput = (index: number, value: string) => {
    // Strip non-digits and get last digit
    const digitsOnly = value.replace(/\D/g, "");
    const digit = digitsOnly.slice(-1);

    const newDigits = [...otpDigits()];
    newDigits[index] = digit;
    setOtpDigits(newDigits);

    // Auto-focus next input
    if (digit && index < 5) {
      const nextInput = document.getElementById(`otp-${index + 1}`);
      nextInput?.focus();
    }
  };

  const handleOtpKeyDown = (index: number, e: KeyboardEvent) => {
    // Handle backspace - clear current and move to previous
    if (e.key === "Backspace") {
      if (otpDigits()[index]) {
        // Clear current digit and move to previous
        const newDigits = [...otpDigits()];
        newDigits[index] = "";
        setOtpDigits(newDigits);
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
    const pastedData = e.clipboardData?.getData("text") || "";
    const digits = pastedData.replace(/\D/g, "").slice(0, 6).split("");

    const newDigits = [...otpDigits()];
    digits.forEach((digit, i) => {
      newDigits[i] = digit;
    });
    setOtpDigits(newDigits);

    // Focus the next empty input or last input
    const nextEmptyIndex = newDigits.findIndex((d) => !d);
    const focusIndex = nextEmptyIndex === -1 ? 5 : nextEmptyIndex;
    document.getElementById(`otp-${focusIndex}`)?.focus();
  };

  const onSubmit = async (e: Event) => {
    e.preventDefault();

    if (!turnstileToken()) {
      toast("Please complete the security verification", "error");
      return;
    }

    const body: CompleteSignUpRequest = {
      username: username(),
      verificationToken: otpDigits().join(""),
      cfTurnstileToken: turnstileToken(),
    };
    const resp = await fetch("/api/auth/join", {
      method: "POST",
      body: JSON.stringify(body),
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (resp.ok) {
      // Handle successful sign-up (e.g., redirect to login or dashboard)
      console.log("Account confirmed successfully");
      navigate("/login");
    } else {
      // Handle sign-up errors
      console.error("Error confirming account:", resp.statusText);
      toast("Error confirming account", "error");
    }
  };

  return (
    <form
      onSubmit={onSubmit}
      class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
    >
      {/* Header */}
      <div class="mb-10 items-center justify-between flex flex-row">
        <h1 class="font-bold text-2xl text-white">Confirm Sign Up</h1>
        <div class="flex flex-row items-end">
          <div class="text-gray-400 font-extralight text-sm mr-2">
            New User?
          </div>
          <A
            class="text-primary font-thin text-sm hover:underline"
            href="/sign-up"
          >
            Sign Up
          </A>
        </div>
      </div>

      {/* Username Input - only show if username was not pre-filled */}
      <Show when={!usernameWasPreFilled}>
        <Input
          type={InputType.Text}
          placeholder="Username"
          value={username}
          onInput={(e) => setUsername(e.currentTarget.value)}
          styleVariant="medium"
        />
      </Show>

      {/* Show username as text if it was pre-filled */}
      <Show when={usernameWasPreFilled}>
        <div class="text-gray-400 text-sm">
          Confirming account for{" "}
          <span class="text-white font-medium">{username()}</span>
        </div>
      </Show>

      {/* OTP Input - 6 digits */}
      <div class="mt-8">
        <p class="text-gray-400 text-xs mb-2">
          Enter the 6-digit code sent to you
        </p>
        <div class="flex gap-3">
          <For each={[0, 1, 2, 3, 4, 5]}>
            {(index) => (
              <Input
                id={`otp-${index}`}
                type={InputType.Tel}
                maxLength={1}
                value={() => otpDigits()[index]}
                onInput={(e) => handleOtpInput(index, e.currentTarget.value)}
                onKeyDown={(e) => handleOtpKeyDown(index, e)}
                onPaste={handleOtpPaste}
                class="flex-1"
                innerClass="text-center text-xl font-medium"
              />
            )}
          </For>
        </div>
      </div>

      {/* Turnstile Widget */}
      <div class="mt-6 flex justify-center">
        <Turnstile
          onVerify={setTurnstileToken}
          onExpire={() => setTurnstileToken("")}
          onError={() => setTurnstileToken("")}
          action="complete-sign-up"
        />
      </div>

      {/* Sign Up Button */}
      <div class="pt-8 w-full flex flex-row items-center justify-between">
        <A
          href="/sign-up"
          class="text-primary text-xs hover:underline font-light"
        >
          Back to Sign Up
        </A>
        <Button
          variant={ButtonVariant.Contained}
          class="py-4 text-base font-semibold px-xxl flex-end transition-all duration-200"
        >
          Confirm
        </Button>
      </div>
    </form>
  );
};

export default ConfirmSignUp;

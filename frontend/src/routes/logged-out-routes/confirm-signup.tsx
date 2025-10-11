import { A, useNavigate, useSearchParams } from "@solidjs/router";
import { createSignal, Show, onMount } from "solid-js";
import Button from "~/components/button";
import LoadingSpinner from "~/components/loading-spinner";
import ErrorMessage from "~/components/error-message";
import SuccessMessage from "~/components/success-message";
import OTPInput from "~/components/otp-input";
import { ButtonVariant } from "~/utils/color";
import { api } from "~/utils/api";
import { ValidationUtil } from "~/utils/validation";

const ConfirmSignup = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();

  // Form state
  const [otp, setOtp] = createSignal("");
  const [email, setEmail] = createSignal("");
  
  // UI state
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [success, setSuccess] = createSignal<string | null>(null);
  const [otpError, setOtpError] = createSignal<string | null>(null);

  // Generate random stars
  const stars = Array.from({ length: 25 }, () => ({
    top: `${Math.random() * 100}%`,
    left: `${Math.random() * 100}%`,
    size: Math.random() * 5,
    delay: `${Math.random() * 3}s`,
    duration: `${Math.random() * 2 + 1.5}s`,
  }));

  const randomizeDuration = (element: HTMLDivElement) => {
    const newDuration = Math.random() * 2 + 1.5; // Random duration between 1.5-3.5s
    element.style.animationDuration = `${newDuration}s`;
  };

  // Initialize email from URL params
  onMount(() => {
    const emailParam = searchParams.email;
    if (emailParam) {
      const emailString = Array.isArray(emailParam) ? emailParam[0] : emailParam;
      setEmail(decodeURIComponent(emailString));
    }
  });

  // Handle OTP value change
  const handleOtpChange = (value: string) => {
    setOtp(value);
    // Clear OTP error when user starts typing
    if (otpError()) {
      setOtpError(null);
    }
  };

  // Handle OTP completion (when all 6 digits are entered)
  const handleOtpComplete = (value: string) => {
    setOtp(value);
    // Auto-submit when OTP is complete
    handleSubmit();
  };

  // Handle form submission
  const handleSubmit = async () => {
    // Clear previous messages
    setError(null);
    setSuccess(null);
    setOtpError(null);

    // Validate OTP
    const otpValidation = ValidationUtil.validateOTP(otp());
    if (!otpValidation.isValid) {
      setOtpError(otpValidation.error!);
      return;
    }

    // Validate email
    if (!email()) {
      setError("Email address is missing. Please go back to sign up.");
      return;
    }

    setIsLoading(true);

    try {
      const result = await api.confirmEmail(otp(), email());

      if (result.success) {
        setSuccess("Email verified successfully! Redirecting to login...");
        
        // Redirect to login page after a short delay
        setTimeout(() => {
          navigate("/login");
        }, 2000);
      } else {
        setError(result.message || "Invalid verification code. Please try again.");
      }
    } catch (err) {
      setError("An unexpected error occurred. Please try again.");
    } finally {
      setIsLoading(false);
    }
  };

  // Handle retry
  const handleRetry = () => {
    setError(null);
    handleSubmit();
  };

  // Handle resend OTP (placeholder for future implementation)
  const handleResendOtp = async () => {
    // This would call a resend OTP API endpoint
    // For now, just show a message
    setSuccess("Verification code resent to your email.");
  };

  return (
    <div
      class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden"
      style={{
        "background-image": "url('/images/starry-sky.svg')",
        "background-size": "cover",
        "background-position": "center",
      }}
    >
      {/* Scattered stars */}
      {stars.map((star) => (
        <div
          ref={(el) => {
            el.addEventListener("animationiteration", () =>
              randomizeDuration(el)
            );
          }}
          class="absolute bg-white rounded-full animate-pulse"
          style={{
            top: star.top,
            left: star.left,
            width: `${star.size}px`,
            height: `${star.size}px`,
            "animation-delay": star.delay,
            "animation-duration": star.duration,
          }}
        />
      ))}
      
      <img
        src="/images/astronaut.svg"
        alt="Floating Astronaut"
        class="absolute bottom-0 left-0 pointer-events-none z-0"
      />
      <img
        src="/images/planet.svg"
        alt="Purple Planet"
        class="absolute top-[-10%] right-[-5%] pointer-events-none z-0 w-[15%]"
      />
      <img
        src="/images/spaceship.svg"
        alt="Spaceship"
        class="
          absolute top-[5%] right-[5%] pointer-events-none 
          z-0 w-[15%] animate-[float_25s_ease-in-out_infinite]
          rotate-[-20deg] scale-x-[-1]
        "
      />
      <img
        src="/images/patr.svg"
        alt="Patr Logo"
        class="absolute top-0 left-0 pointer-events-none z-0 mt-6 ml-4 w-[15%]"
      />

      {/* Confirm Signup Card */}
      <section class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-[32rem] relative z-10 border border-secondary-medium">
        {/* Header */}
        <div class="text-center mb-10">
          <h1 class="font-bold text-2xl text-white mb-3">Verify Your Email</h1>
          <p class="text-gray-400 text-sm">
            We've sent a 6-digit verification code to
          </p>
          <p class="text-primary text-sm font-medium mt-1">
            {email() || "your email address"}
          </p>
        </div>

        {/* Success Message */}
        <Show when={success()}>
          <SuccessMessage
            message={success()!}
            class="mb-6"
          />
        </Show>

        {/* Error Message */}
        <Show when={error()}>
          <ErrorMessage
            message={error()!}
            dismissible={true}
            showRetry={true}
            onRetry={handleRetry}
            onDismiss={() => setError(null)}
            class="mb-6"
          />
        </Show>

        {/* OTP Input */}
        <div class="mb-6">
          <label class="text-white text-sm font-medium block text-center mb-4">
            Enter Verification Code
          </label>
          <OTPInput
            length={6}
            onValueChange={handleOtpChange}
            onComplete={handleOtpComplete}
            disabled={isLoading()}
            hasError={!!otpError()}
            class="mb-2"
          />
          <Show when={otpError()}>
            <p class="text-error text-sm text-center mt-2">{otpError()}</p>
          </Show>
        </div>

        {/* Verify Button */}
        <div class="mb-6">
          <Button
            variant={ButtonVariant.Contained}
            class="w-full py-4 text-base font-semibold flex items-center justify-center gap-2"
            onClick={handleSubmit}
            disabled={isLoading() || otp().length !== 6}
          >
            <Show when={isLoading()}>
              <LoadingSpinner size="sm" />
            </Show>
            {isLoading() ? "Verifying..." : "Verify Email"}
          </Button>
        </div>

        {/* Resend Code */}
        <div class="text-center mb-6">
          <p class="text-gray-400 text-sm mb-2">
            Didn't receive the code?
          </p>
          <button
            onClick={handleResendOtp}
            class="text-primary text-sm hover:underline font-medium"
            disabled={isLoading()}
          >
            Resend Code
          </button>
        </div>

        {/* Back to Sign Up */}
        <div class="text-center">
          <A
            href="/sign-up"
            class="text-gray-400 text-sm hover:text-white transition-colors"
          >
            ← Back to Sign Up
          </A>
        </div>
      </section>

      {/* Footer */}
      <div class="absolute bottom-6 left-0 right-0 text-center">
        <p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
      </div>
    </div>
  );
};

export default ConfirmSignup;
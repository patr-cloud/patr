import { A, useNavigate } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import Button from "~/components/button";
import Input, { InputVariants } from "~/components/input";
import LoadingSpinner from "~/components/loading-spinner";
import ErrorMessage from "~/components/error-message";
import SuccessMessage from "~/components/success-message";
import { ButtonVariant } from "~/utils/color";
import { api, type SignupData } from "~/utils/api";
import { ValidationUtil } from "~/utils/validation";

const SignUp = () => {
  const navigate = useNavigate();

  // Form state
  const [firstName, setFirstName] = createSignal("");
  const [lastName, setLastName] = createSignal("");
  const [username, setUsername] = createSignal("");
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [confirmPassword, setConfirmPassword] = createSignal("");
  
  // UI state
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [success, setSuccess] = createSignal<string | null>(null);
  const [fieldErrors, setFieldErrors] = createSignal<{
    firstName?: string;
    lastName?: string;
    username?: string;
    email?: string;
    password?: string;
    confirmPassword?: string;
  }>({});

  // Validation functions
  const validateForm = () => {
    const errors: typeof fieldErrors extends () => infer T ? T : never = {};

    // Validate first name
    const firstNameValidation = ValidationUtil.validateRequired(firstName(), "First name");
    if (!firstNameValidation.isValid) {
      errors.firstName = firstNameValidation.error;
    }

    // Validate last name
    const lastNameValidation = ValidationUtil.validateRequired(lastName(), "Last name");
    if (!lastNameValidation.isValid) {
      errors.lastName = lastNameValidation.error;
    }

    // Validate username
    const usernameValidation = ValidationUtil.validateUsername(username());
    if (!usernameValidation.isValid) {
      errors.username = usernameValidation.error;
    }

    // Validate email
    const emailValidation = ValidationUtil.validateEmail(email());
    if (!emailValidation.isValid) {
      errors.email = emailValidation.error;
    }

    // Validate password
    const passwordValidation = ValidationUtil.validatePassword(password());
    if (!passwordValidation.isValid) {
      errors.password = passwordValidation.error;
    }

    // Validate confirm password
    const confirmPasswordValidation = ValidationUtil.validateConfirmPassword(password(), confirmPassword());
    if (!confirmPasswordValidation.isValid) {
      errors.confirmPassword = confirmPasswordValidation.error;
    }

    setFieldErrors(errors);
    return Object.keys(errors).length === 0;
  };

  // Handle form submission
  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    
    // Clear previous messages
    setError(null);
    setSuccess(null);

    // Validate form
    if (!validateForm()) {
      return;
    }

    setIsLoading(true);

    try {
      const signupData: SignupData = {
        username: username().trim(),
        email: email().trim(),
        password: password(),
        confirmPassword: confirmPassword(),
      };

      const result = await api.signup(signupData);

      if (result.success) {
        setSuccess("Account created successfully! Please check your email for verification.");
        
        // Redirect to confirm signup page after a short delay
        setTimeout(() => {
          navigate(`/confirm-signup?email=${encodeURIComponent(result.email)}`);
        }, 2000);
      } else {
        setError(result.message || "Failed to create account. Please try again.");
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
    handleSubmit(new Event('submit'));
  };

  // Clear field error when user starts typing
  const clearFieldError = (fieldName: string) => {
    const currentErrors = fieldErrors();
    if (currentErrors[fieldName as keyof typeof currentErrors]) {
      setFieldErrors(prev => ({ ...prev, [fieldName]: undefined }));
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden"
      style={{
        "background-image": "url('/images/starry-sky.svg')",
        "background-size": "cover",
        "background-position": "center",
      }}
    >
      {/* Background decorative elements */}
      <div class="absolute inset-0 overflow-hidden pointer-events-none">
        <div class="absolute inset-0 w-full h-full bg-gradient-to-br from-secondary via-secondary-dark to-secondary opacity-50"></div>
      </div>

      {/* Sign Up Card */}
      <section class="bg-secondary-dark p-12 rounded-2xl shadow-2xl w-full max-w-[520px] relative z-10 border border-secondary-medium">
        {/* Logo */}
        <div class="flex justify-center mb-8">
          <div class="text-primary text-4xl font-bold">PATR</div>
        </div>

        {/* Header */}
        <div class="text-center mb-10">
          <h1 class="text-4xl font-bold text-white mb-3">Create Account</h1>
          <p class="text-gray-400 text-base">Join Patr to get started</p>
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

        {/* Form */}
        <div class="space-y-5">
          {/* First Name and Last Name Row */}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* First Name Input */}
            <div class="space-y-2">
              <label class="text-white text-sm font-medium block pl-1">
                First Name
              </label>
              <Input
                type={InputVariants.Text}
                placeholder="Enter your first name"
                value={firstName()}
                onInput={(e: Event) => {
                  const target = e.currentTarget as HTMLInputElement;
                  setFirstName(target.value);
                  clearFieldError('firstName');
                }}
                styleVariant="medium"
                disabled={isLoading()}
              />
              <Show when={fieldErrors().firstName}>
                <p class="text-error text-sm mt-1 ml-1">{fieldErrors().firstName}</p>
              </Show>
            </div>

            {/* Last Name Input */}
            <div class="space-y-2">
              <label class="text-white text-sm font-medium block pl-1">
                Last Name
              </label>
              <Input
                type={InputVariants.Text}
                placeholder="Enter your last name"
                value={lastName()}
                onInput={(e: Event) => {
                  const target = e.currentTarget as HTMLInputElement;
                  setLastName(target.value);
                  clearFieldError('lastName');
                }}
                styleVariant="medium"
                disabled={isLoading()}
              />
              <Show when={fieldErrors().lastName}>
                <p class="text-error text-sm mt-1 ml-1">{fieldErrors().lastName}</p>
              </Show>
            </div>
          </div>

          {/* Username Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Username
            </label>
            <Input
              type={InputVariants.Text}
              placeholder="Choose a username"
              value={username()}
              onInput={(e: Event) => {
                const target = e.currentTarget as HTMLInputElement;
                setUsername(target.value);
                clearFieldError('username');
              }}
              styleVariant="medium"
              disabled={isLoading()}
            />
            <Show when={fieldErrors().username}>
              <p class="text-error text-sm mt-1 ml-1">{fieldErrors().username}</p>
            </Show>
          </div>

          {/* Email Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Email Address
            </label>
            <Input
              type={InputVariants.Email}
              placeholder="Enter your email"
              value={email()}
              onInput={(e: Event) => {
                const target = e.currentTarget as HTMLInputElement;
                setEmail(target.value);
                clearFieldError('email');
              }}
              styleVariant="medium"
              disabled={isLoading()}
            />
            <Show when={fieldErrors().email}>
              <p class="text-error text-sm mt-1 ml-1">{fieldErrors().email}</p>
            </Show>
          </div>

          {/* Password Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Password
            </label>
            <Input
              type={InputVariants.Password}
              placeholder="Create a password"
              value={password()}
              onInput={(e: Event) => {
                const target = e.currentTarget as HTMLInputElement;
                setPassword(target.value);
                clearFieldError('password');
              }}
              styleVariant="medium"
              disabled={isLoading()}
            />
            <Show when={fieldErrors().password}>
              <p class="text-error text-sm mt-1 ml-1">{fieldErrors().password}</p>
            </Show>
          </div>

          {/* Confirm Password Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Confirm Password
            </label>
            <Input
              type={InputVariants.Password}
              placeholder="Confirm your password"
              value={confirmPassword()}
              onInput={(e: Event) => {
                const target = e.currentTarget as HTMLInputElement;
                setConfirmPassword(target.value);
                clearFieldError('confirmPassword');
              }}
              styleVariant="medium"
              disabled={isLoading()}
            />
            <Show when={fieldErrors().confirmPassword}>
              <p class="text-error text-sm mt-1 ml-1">{fieldErrors().confirmPassword}</p>
            </Show>
          </div>

          {/* Sign Up Button */}
          <div class="pt-4">
            <Button
              variant={ButtonVariant.Contained}
              class="w-full py-4 text-base font-semibold flex items-center justify-center gap-2"
              type="submit"
              disabled={isLoading()}
            >
              <Show when={isLoading()}>
                <LoadingSpinner size="sm" />
              </Show>
              {isLoading() ? "Creating Account..." : "Create Account"}
            </Button>
          </div>
        </div>

        {/* Divider */}
        <div class="flex items-center my-8">
          <div class="flex-1 border-t border-gray-600"></div>
          <span class="px-4 text-gray-500 text-xs uppercase tracking-wider">
            OR
          </span>
          <div class="flex-1 border-t border-gray-600"></div>
        </div>

        {/* Sign In Link */}
        <div class="text-center">
          <p class="text-gray-400 text-sm">
            Already have an account?{" "}
            <A href="/login" class="text-primary font-semibold hover:underline">
              Sign In
            </A>
          </p>
        </div>
      </section>

      {/* Footer */}
      <div class="absolute bottom-6 left-0 right-0 text-center">
        <p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
      </div>
    </form>
  );
};

export default SignUp;

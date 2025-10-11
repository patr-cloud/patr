import { A, useNavigate } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import Button from "~/components/button";
import Input, { InputVariants } from "~/components/input";
import LoadingSpinner from "~/components/loading-spinner";
import ErrorMessage from "~/components/error-message";
import { ButtonVariant } from "~/utils/color";
import { useAuthState } from "~/utils/state";
import { api, type LoginCredentials } from "~/utils/api";
import { ValidationUtil } from "~/utils/validation";

const Login = () => {
  const navigate = useNavigate();
  const [_, setAuthState] = useAuthState();

  // Form state
  const [userId, setUserId] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [fieldErrors, setFieldErrors] = createSignal<{
    userId?: string;
    password?: string;
  }>({});

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

  // Validation functions
  const validateUserId = (value: string) => {
    if (!value.trim()) {
      return "Username or email is required";
    }
    return null;
  };

  const validatePassword = (value: string) => {
    if (!value.trim()) {
      return "Password is required";
    }
    return null;
  };

  // Handle form submission
  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    
    // Clear previous errors
    setError(null);
    setFieldErrors({});

    // Validate form
    const userIdError = validateUserId(userId());
    const passwordError = validatePassword(password());

    if (userIdError || passwordError) {
      setFieldErrors({
        userId: userIdError || undefined,
        password: passwordError || undefined,
      });
      return;
    }

    setIsLoading(true);

    try {
      const credentials: LoginCredentials = {
        userId: userId().trim(),
        password: password(),
      };

      const result = await api.login(credentials);

      if (result.success) {
        // Store tokens in auth state (which uses cookies)
        api.setAuthTokens({
          accessToken: result.accessToken,
          refreshToken: result.refreshToken,
        });
        
        // Redirect to create workspace page
        navigate("/create-workspace");
      } else {
        setError(result.message || "Login failed. Please try again.");
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

      {/* Login Card */}
      <section class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-[32rem] relative z-10 border border-secondary-medium">
        {/* Header */}
        <div class="mb-10 items-center justify-between flex flex-row">
          <h1 class="font-bold text-2xl text-white">Login</h1>
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
        <div>
          <div>
            <Input
              type={InputVariants.Text}
              placeholder="Username or Email"
              name="userId"
              value={userId()}
              onInput={(e) => {
                const target = e.currentTarget as HTMLInputElement;
                setUserId(target.value);
                // Clear field error when user starts typing
                if (fieldErrors().userId) {
                  setFieldErrors(prev => ({ ...prev, userId: undefined }));
                }
              }}
              class={() => "mt-4"}
              styleVariant="medium"
              disabled={isLoading()}
            />
            <Show when={fieldErrors().userId}>
              <p class="text-error text-sm mt-1 ml-1">{fieldErrors().userId}</p>
            </Show>
          </div>

          <div>
            <Input
              type={InputVariants.Password}
              placeholder="Password"
              name="password"
              value={password()}
              onInput={(e) => {
                const target = e.currentTarget as HTMLInputElement;
                setPassword(target.value);
                // Clear field error when user starts typing
                if (fieldErrors().password) {
                  setFieldErrors(prev => ({ ...prev, password: undefined }));
                }
              }}
              class={() => "mt-4"}
              styleVariant="medium"
              disabled={isLoading()}
            />
            <Show when={fieldErrors().password}>
              <p class="text-error text-sm mt-1 ml-1">{fieldErrors().password}</p>
            </Show>
          </div>

          {/* Login Button */}
          <div class="pt-8 w-full flex flex-row items-center justify-between">
            <A
              href="/forgot-password"
              class="text-primary text-xs hover:underline font-light"
            >
              Forgot password?
            </A>
            <Button
              variant={ButtonVariant.Contained}
              class="py-4 text-base font-semibold px-xxl flex-end transition-all duration-200 flex items-center gap-2"
              type="submit"
              disabled={isLoading()}
            >
              <Show when={isLoading()}>
                <LoadingSpinner size="sm" />
              </Show>
              {isLoading() ? "Logging in..." : "Login"}
            </Button>
          </div>
        </div>
      </section>

      {/* Footer */}
      <div class="absolute bottom-6 left-0 right-0 text-center">
        <p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
      </div>
    </form>
  );
};

export default Login;

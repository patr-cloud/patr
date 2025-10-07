import { A } from "@solidjs/router";
import { createSignal } from "solid-js";
import Button from "~/components/button";
import Input, { InputVariants } from "~/components/input";
import { ButtonVariant } from "~/utils/color";

const SignUp = () => {
  const [name, setName] = createSignal("");
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [confirmPassword, setConfirmPassword] = createSignal("");

  return (
    <main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden">
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

        {/* Form */}
        <div class="space-y-5">
          {/* Name Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Full Name
            </label>
            <Input
              type={InputVariants.Text}
              placeholder="Enter your full name"
              value={name}
              onInput={(e: Event) =>
                setName((e.currentTarget as HTMLInputElement).value)
              }
              styleVariant="medium"
            />
          </div>

          {/* Email Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Email Address
            </label>
            <Input
              type={InputVariants.Email}
              placeholder="Enter your email"
              value={email}
              onInput={(e: Event) =>
                setEmail((e.currentTarget as HTMLInputElement).value)
              }
              styleVariant="medium"
            />
          </div>

          {/* Password Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Password
            </label>
            <Input
              type={InputVariants.Password}
              placeholder="Create a password"
              value={password}
              onInput={(e: Event) =>
                setPassword((e.currentTarget as HTMLInputElement).value)
              }
              styleVariant="medium"
            />
          </div>

          {/* Confirm Password Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Confirm Password
            </label>
            <Input
              type={InputVariants.Password}
              placeholder="Confirm your password"
              value={confirmPassword}
              onInput={(e: Event) =>
                setConfirmPassword((e.currentTarget as HTMLInputElement).value)
              }
              styleVariant="medium"
            />
          </div>

          {/* Sign Up Button */}
          <div class="pt-4">
            <Button
              variant={ButtonVariant.Contained}
              class="w-full py-4 text-base font-semibold"
            >
              Create Account
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
    </main>
  );
};

export default SignUp;

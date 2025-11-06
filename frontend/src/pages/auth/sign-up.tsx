import { A, redirect } from "@solidjs/router";
import { createSignal } from "solid-js";
import { CreateAccountRequest } from "~/bindings";
import Button from "~/components/button";
import Input, { InputType } from "~/components/input";
import { ButtonVariant } from "~/utils/color";
import { doFetch } from "~/utils/do-fetch";

const SignUp = () => {
  const [username, setUsername] = createSignal("");
  const [firstName, setFirstName] = createSignal("");
  const [lastName, setLastName] = createSignal("");
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [confirmPassword, setConfirmPassword] = createSignal("");

  const onSubmit = async (e: Event) => {
    e.preventDefault();
    // Handle sign-up logic here

    const requestBody: CreateAccountRequest = {
      username: username(),
      password: password(),
      firstName: firstName(),
      lastName: lastName(),
      recoveryEmail: email(),
    };

    console.log("Sign Up Request:", requestBody);
    const resp = await doFetch("/api/auth/sign-up", {
      method: "POST",
      body: JSON.stringify(requestBody),
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (resp.ok) {
      // Handle successful sign-up (e.g., redirect to login or dashboard)
      console.log("Account created successfully");
      throw redirect("/confirm-signup");
    } else {
      // Handle sign-up errors
      console.error("Error creating account:", resp.statusText);
      alert("Error creating account: " + resp.statusText);
    }
  };

  return (
    <>
      {/* Sign Up Card */}
      <section class="bg-secondary p-12 rounded-2xl shadow-2xl w-full max-w-[520px] relative z-10 border border-secondary-medium">
        {/* Logo */}
        <div class="flex justify-center mb-6">
          <div class="text-primary text-4xl font-bold">PATR</div>
        </div>

        {/* Header */}
        <div class="text-center mb-8">
          <h1 class="text-4xl font-bold text-white mb-3">Create Account</h1>
          <p class="text-gray-400 text-base">Join Patr to get started</p>
        </div>

        {/* Form */}
        <form onSubmit={onSubmit} class="space-y-5">
          {/* Username Input */}
          <div class="space-y-2">
            <label
              for="username"
              class="text-white text-sm font-medium block pl-1"
            >
              User name
            </label>
            <Input
              type={InputType.Text}
              placeholder="Enter your username"
              name="username"
              id="username"
              value={username}
              onInput={(e) => setUsername(e.currentTarget.value)}
              styleVariant="medium"
            />
          </div>

          {/* Name Input */}
          <div class="flex items-center justify-center gap-4">
            <div class="flex-6 mb-0 space-y-2">
              <label
                for="first-name"
                class="text-white text-sm font-medium block pl-1"
              >
                First Name
              </label>
              <Input
                type={InputType.Text}
                placeholder="Enter your first name"
                required={true}
                name="first-name"
                id="first-name"
                value={firstName}
                onInput={(e) => setFirstName(e.currentTarget.value)}
                styleVariant="medium"
              />
            </div>

            <div class="flex-6 space-y-2">
              <label
                for="last-name"
                class="text-white text-sm font-medium block pl-1"
              >
                Last Name
              </label>
              <Input
                type={InputType.Text}
                placeholder="Enter your last name"
                required={true}
                name="last-name"
                id="last-name"
                value={lastName}
                onInput={(e) => setLastName(e.currentTarget.value)}
                styleVariant="medium"
              />
            </div>
          </div>

          {/* Email Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Email Address
            </label>
            <Input
              type={InputType.Email}
              placeholder="Enter your email"
              value={email}
              onInput={(e) => setEmail(e.currentTarget.value)}
              styleVariant="medium"
            />
          </div>

          {/* Password Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Password
            </label>
            <Input
              type={InputType.Password}
              placeholder="Create a password"
              value={password}
              onInput={(e) => setPassword(e.currentTarget.value)}
              styleVariant="medium"
            />
          </div>

          {/* Confirm Password Input */}
          <div class="space-y-2">
            <label class="text-white text-sm font-medium block pl-1">
              Confirm Password
            </label>
            <Input
              type={InputType.Password}
              placeholder="Confirm your password"
              value={confirmPassword}
              onInput={(e) => setConfirmPassword(e.currentTarget.value)}
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
        </form>

        <div class="pt-4">
          <A
            href="/confirm-signup"
            class="w-full text-white py-4 text-base font-semibold"
          >
            Have an OTP?
          </A>
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
    </>
  );
};

export default SignUp;

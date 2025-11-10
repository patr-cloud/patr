import { A, redirect } from "@solidjs/router";
import { createSignal } from "solid-js";
import { CreateAccountRequest } from "~/bindings";
import Button from "~/components/button";
import Input, { InputType } from "~/components/input";
import { ButtonVariant } from "~/utils/color";
import { httpRequest } from "~/utils/http-request";

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
    const resp = await httpRequest("/api/auth/sign-up", {
      method: "POST",
      body: JSON.stringify(requestBody),
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (resp.ok) {
      // Handle successful sign-up (e.g., redirect to login or dashboard)
      console.log("Account created successfully");
      redirect("/confirm-signup?username=" + encodeURIComponent(username()));
    } else {
      // Handle sign-up errors
      console.error("Error creating account:", resp.statusText);
      alert("Error creating account: " + resp.statusText);
    }
  };

  return (
    <>
      {/* Sign Up Card */}
      <form
        onSubmit={onSubmit}
        class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-[32rem] relative z-10 border border-secondary-medium"
      >
        {/* Header */}
        <div class="mb-10 items-center justify-between flex flex-row">
          <h1 class="font-bold text-2xl text-white">Sign Up</h1>
          <div class="flex flex-row items-end">
            <div class="text-gray-400 font-extralight text-sm mr-2">
              Already a User?
            </div>
            <A
              class="text-primary font-thin text-sm hover:underline"
              href="/login"
            >
              Login
            </A>
          </div>
        </div>

        {/* Form */}
        <div>
          <Input
            type={InputType.Text}
            placeholder="Username"
            name="username"
            id="username"
            value={username}
            onInput={(e) => setUsername(e.currentTarget.value)}
            styleVariant="medium"
          />

          {/* Name Inputs */}
          <div class="flex items-center gap-4 mt-4">
            <Input
              type={InputType.Text}
              placeholder="First Name"
              required={true}
              name="first-name"
              id="first-name"
              value={firstName}
              onInput={(e) => setFirstName(e.currentTarget.value)}
              styleVariant="medium"
              class="flex-1"
            />
            <Input
              type={InputType.Text}
              placeholder="Last Name"
              required={true}
              name="last-name"
              id="last-name"
              value={lastName}
              onInput={(e) => setLastName(e.currentTarget.value)}
              styleVariant="medium"
              class="flex-1"
            />
          </div>

          <Input
            type={InputType.Email}
            placeholder="Email Address"
            value={email}
            onInput={(e) => setEmail(e.currentTarget.value)}
            class="mt-4"
            styleVariant="medium"
          />

          <Input
            type={InputType.Password}
            placeholder="Password"
            value={password}
            onInput={(e) => setPassword(e.currentTarget.value)}
            class="mt-4"
            styleVariant="medium"
          />

          <Input
            type={InputType.Password}
            placeholder="Confirm Password"
            value={confirmPassword}
            onInput={(e) => setConfirmPassword(e.currentTarget.value)}
            class="mt-4"
            styleVariant="medium"
          />

          {/* Sign Up Button */}
          <div class="pt-8 w-full flex flex-row items-center justify-between">
            <A
              href="/confirm-signup"
              class="text-primary text-xs hover:underline font-light"
            >
              Have an OTP?
            </A>
            <Button
              variant={ButtonVariant.Contained}
              class="py-4 text-base font-semibold px-xxl flex-end"
              type="submit"
            >
              Sign Up
            </Button>
          </div>
        </div>
      </form>

      {/* Footer */}
      <div class="absolute bottom-6 left-0 right-0 text-center">
        <p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
      </div>
    </>
  );
};

export default SignUp;

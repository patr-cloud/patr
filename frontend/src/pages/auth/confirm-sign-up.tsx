import { A, redirect } from "@solidjs/router";
import { createSignal } from "solid-js";
import { CompleteSignUpRequest } from "~/bindings";
import { Button, ButtonVariant, Input, InputType } from "~/components";

const ConfirmSignUp = () => {
  const [username, setUsername] = createSignal("");
  const [otp, setOtp] = createSignal("");

  const onSubmit = async (e: Event) => {
    e.preventDefault();
    // Handle sign-up logic here
    const body: CompleteSignUpRequest = {
      username: username(),
      verificationToken: otp(),
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
      redirect("/login");
    } else {
      // Handle sign-up errors
      console.error("Error confirming account:", resp.statusText);
    }
  };

  return (
    <form
      onSubmit={onSubmit}
      class="space-y-5 bg-secondary p-12 rounded-2xl shadow-2xl w-full max-w-[520px] relative z-10 border border-secondary-medium"
    >
      {/* Header */}
      <div class="mb-5 items-center justify-between flex flex-row">
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

      {/* Username Input */}
      <div class="space-y-2">
        <label class="text-white text-sm font-medium block pl-1">
          Username
        </label>
        <Input
          type={InputType.Text}
          placeholder="Enter your username"
          value={username}
          onInput={(e) => setUsername(e.currentTarget.value)}
          styleVariant="medium"
        />
      </div>

      {/* Confirm OTP */}
      <div class="space-y-2">
        <label class="text-white text-sm font-medium block pl-1">
          Confirm Password
        </label>
        <Input
          type={InputType.Text}
          placeholder="Confirm your OTP"
          value={otp}
          onInput={(e) => setOtp(e.currentTarget.value)}
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
  );
};

export default ConfirmSignUp;

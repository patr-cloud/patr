import { A, query, redirect } from "@solidjs/router";
import { Button } from "~/components";
import { InputType, BgOnboard, Input } from "~/components";
import { ButtonVariant } from "~/utils/color";
import { JSX } from "solid-js";
import { getRequestEvent } from "solid-js/web";
import { LoginRequest, LoginResponse } from "~/bindings";

const loginFn = query(async (data: LoginRequest) => {
  "use server";

  const event = getRequestEvent();

  if (!event) throw new Error("Expect Request Event");

  const userAgent = event.request.headers.get("user-agent");

  const loginResponse = await fetch("http://localhost:3001/api/auth/sign-in", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "User-Agent": userAgent || "",
    },
    body: JSON.stringify({
      userId: data.userId,
      password: data.password,
      mfaOtp: data.mfaOtp,
    }),
  }).then((res) => res.json() as Promise<LoginResponse>);

  console.log("Login Response Status:", loginResponse);

  /// FIXME: use the hook
  event.response.headers.append(
    "Set-Cookie",
    `authState=${JSON.stringify({
      type: "LoggedIn",
      accessToken: loginResponse.accessToken,
      refreshToken: loginResponse.refreshToken,
    })};Path=/;SameSite=Strict;Max-Age=604800` // 7 days
  );

  // Don't mind the throw, it's just to redirect after setting the cookie
  // and apparently that's how it's done in solid-start and TS
  throw redirect("/");
}, "login");

const Login = () => {
  const onSubmitLogin: JSX.EventHandler<HTMLFormElement, SubmitEvent> = async (
    e
  ) => {
    e.preventDefault();
    const formData = new FormData(e.target as HTMLFormElement);
    const userId = formData.get("userId") as string;
    const password = formData.get("password") as string;

    // Handle login logic here
    console.log("Logging in with", { userId });

    await loginFn({
      userId,
      password,
      mfaOtp: "123456",
    });
  };

  return (
    <main
      class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden"
      style={{
        "background-image": "url('/images/starry-sky.svg')",
        "background-size": "cover",
        "background-position": "center",
      }}
    >
      <BgOnboard />

      {/* Login Card */}
      <form
        onSubmit={onSubmitLogin}
        class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-[32rem] relative z-10 border border-secondary-medium"
      >
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

        {/* Form */}
        <div>
          <Input
            type={InputType.Text}
            placeholder="Username or Email"
            name="userId"
            class={() => "mt-4"}
            styleVariant="medium"
          />

          <Input
            type={InputType.Password}
            placeholder="Password"
            name="password"
            class={() => "mt-4"}
            styleVariant="medium"
          />

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
              class="py-4 text-base font-semibold px-xxl flex-end transition-all duration-200"
              type="submit"
            >
              Login
            </Button>
          </div>
        </div>
      </form>

      {/* Footer */}
      <div class="absolute bottom-6 left-0 right-0 text-center">
        <p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
      </div>
    </main>
  );
};

export default Login;

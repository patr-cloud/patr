import { A, query, redirect, useNavigate } from "@solidjs/router";
import { Button } from "~/components";
import { InputType, Input } from "~/components";
import { ButtonVariant } from "~/utils/color";
import { JSX } from "solid-js";
import { getRequestEvent } from "solid-js/web";
import { LoginRequest, LoginResponse } from "~/bindings";
import { doFetch } from "~/utils/do-fetch";
import { useAuthState } from "~/hooks";

/**
 * @deprecated Not using a server function, cause that limits Client Side Only Capabilities,
 * keeping it here in case we decide to backtrack on it later.
 */
const loginFn = query(async (data: LoginRequest) => {
  "use server";

  const event = getRequestEvent();
  if (!event) throw new Error("Expect Request Event");

  const userAgent = event.request.headers.get("user-agent");

  const loginResponse = await doFetch<LoginResponse>(
    `${import.meta.env.VITE_BASE_URL}/api/auth/sign-in`,
    {
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
    }
  );

  console.log("Login Response Status:", loginResponse);
  if (loginResponse.ok) {
    event.response.headers.append(
      "Set-Cookie",
      `authState=${JSON.stringify({
        type: "LoggedIn",
        accessToken: loginResponse.data.accessToken,
        refreshToken: loginResponse.data.refreshToken,
      })};Path=/;SameSite=Strict;Max-Age=604800` // 7 days
    );

    // Don't mind the throw, it's just to redirect after setting the cookie
    // and apparently that's how it's done in solid-start and TS
    throw redirect("/");
  } else {
    console.error("Login failed:", loginResponse.statusText);
    event.response.status = 401;
    // FIXME: as well so this needs some proper error handling, for now just return false
    return false;
  }
}, "login");

const Login = () => {
  const [, setAuthState] = useAuthState();
  const navigate = useNavigate();

  const onSubmitLogin: JSX.EventHandler<HTMLFormElement, SubmitEvent> = async (
    e
  ) => {
    e.preventDefault();

    const formData = new FormData(e.target as HTMLFormElement);
    const userId = formData.get("userId") as string;
    const password = formData.get("password") as string;

    // Handle login logic here
    console.log("Logging in with", { userId });

    const loginResp = await doFetch<LoginResponse>("/api/auth/sign-in", {
      method: "POST",
      body: JSON.stringify({
        userId,
        password,
      }),
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (loginResp.ok) {
      console.log("Login successful");
      setAuthState({
        type: "LoggedIn",
        accessToken: loginResp.data.accessToken,
        refreshToken: loginResp.data.refreshToken,
      });
      navigate("/", { replace: true });
    } else {
      console.error("Error during login:", loginResp.statusText);
      alert("Error during login: " + loginResp.statusText);
      return;
    }
  };

  return (
    <>
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
            class="mt-4"
            styleVariant="medium"
          />

          <Input
            type={InputType.Password}
            placeholder="Password"
            name="password"
            class="mt-4"
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
    </>
  );
};

export default Login;

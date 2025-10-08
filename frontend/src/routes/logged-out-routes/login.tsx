import { createServerCookie } from "@solid-primitives/cookies";
import { A, action, redirect } from "@solidjs/router";
import Button from "~/components/button";
import Input, { InputVariants } from "~/components/input";
import { ButtonVariant } from "~/utils/color";

const loginAction = action(async (formData: FormData) => {
  "use server";
  const userId = formData.get("userId") as string;
  const password = formData.get("password") as string;

  const [_, setAuthState] = createServerCookie("authState");

  // Handle login logic here
  console.log("Logging in with", { userId, password });

  if (userId === "user" && password === "password") {
    // Mock authentication success
    setAuthState("loggedIn");
    return redirect("/");
  } else {
    // Mock authentication failure
    return new Response("Invalid credentials", { status: 401 });
  }
});

const Login = () => {
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

  return (
    <form
      action={loginAction}
      method="post"
      class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden"
      style={{
        "background-image": "url('/images/starry-sky.svg')",
        "background-size": "cover",
        "background-position": "center",
      }}
    >
      {/* Scattered stars */}
      {stars.map((star, i) => (
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

        {/* Form */}
        <div>
          <Input
            type={InputVariants.Text}
            placeholder="Username or Email"
            name="userId"
            class={() => "mt-4"}
            styleVariant="medium"
          />

          <Input
            type={InputVariants.Password}
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
      </section>

      {/* Footer */}
      <div class="absolute bottom-6 left-0 right-0 text-center">
        <p class="text-gray-500 text-xs">© 2025 Patr. All rights reserved.</p>
      </div>
    </form>
  );
};

export default Login;

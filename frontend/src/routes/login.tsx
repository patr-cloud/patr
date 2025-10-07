import { A } from "@solidjs/router";
import Button from "~/components/button";
import Input, { InputVariants } from "~/components/input";
import { ButtonVariant } from "~/utils/color";

const Login = () => {
  return (
    <main
      class="
        h-screen min-h-screen w-screen min-w-screen bg-secondary 
        flex items-center justify-center
      "
    >
      <section class="bg-secondary-dark p-13 rounded-md shadow-lg w-128">
        <div class="text-primary flex items-center justify-between">
          <h1 class="text-xl font-medium">Sign In</h1>
          <div class="flex items-center font-thin text-xs justify-center gap-xs">
            <p class="text-white">New User?</p>
            <A href="/sign-up">Sign Up</A>
          </div>
        </div>
        <Input
          type={InputVariants.Text}
          onInput={(e: Event) =>
            console.log((e.currentTarget as HTMLInputElement).value)
          }
        />
        <Input
          type={InputVariants.Password}
          onInput={(e: Event) =>
            console.log((e.currentTarget as HTMLInputElement).value)
          }
        />
        <Button variant={ButtonVariant.Contained}>Login</Button>
      </section>
    </main>
  );
};

export default Login;

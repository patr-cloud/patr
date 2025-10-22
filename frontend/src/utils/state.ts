import { createServerCookie } from "@solid-primitives/cookies";
import { Signal } from "solid-js";

/// The authentication state of the user. This is what gets stored in the cookie
export type AuthState =
  | {
      type: "LoggedIn";
      accessToken: string;
      refreshToken: string;
    }
  | {
      type: "LoggedOut";
    };

export function useAuthState(): Signal<AuthState> {
  return createServerCookie<AuthState>("authState", {
    serialize: (value) => {
      console.log(value);
      return JSON.stringify(value);
    },
    deserialize: (value) =>
      value
        ? JSON.parse(value)
        : {
            type: "LoggedOut",
          },
  });
}

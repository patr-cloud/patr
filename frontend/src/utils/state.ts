import { createServerCookie } from "@solid-primitives/cookies";
import {
  makePersisted,
  cookieStorage,
  CookieOptions,
} from "@solid-primitives/storage";
import { createSignal, Signal } from "solid-js";

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
  const [getter, setter] = makePersisted(
    createSignal<AuthState>({ type: "LoggedOut" }),
    {
      storage: cookieStorage.withOptions({
        expires: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7), // 7 days
        path: "/",
        sameSite: "Lax",
      }),
    }
  );

  return [getter, setter];
}

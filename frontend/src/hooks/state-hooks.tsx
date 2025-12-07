import { makePersisted, cookieStorage } from "@solid-primitives/storage";
import {
	createContext,
	createSignal,
	ParentProps,
	Signal,
	useContext,
} from "solid-js";

const AuthStateContext = createContext<Signal<AuthState | null>>();
const LastWorkspaceIdContext = createContext<Signal<string | null>>();

/// The authentication state of the user. This is what gets stored in the cookie
export type AuthState =
	| {
			type: "LoggedIn";
			accessToken: string;
			refreshToken: string;
	  }
	| {
			type: "LoggedOut";
	  }
	| null;

/**
 * A Component that provides the AuthState context to its children
 */
export const AuthStateProvider = (props: ParentProps<{}>) => {
	const [authState, setAuthState] = makePersisted(
		createSignal<AuthState | null>(null),
		{
			name: "authState",
			storage: cookieStorage.withOptions({
				expires: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7), // 7 days
				path: "/",
				sameSite: "Strict",
			}),
		}
	);

	return (
		<AuthStateContext.Provider value={[authState, setAuthState]}>
			{props.children}
		</AuthStateContext.Provider>
	);
};

export function useAuthState(): Signal<AuthState | null> {
	const signal = useContext(AuthStateContext);
	if (!signal) {
		throw new Error("useAuthState must be used within an AuthStateProvider");
	}

	return signal;
}

/**
 * A Component that provides the LastWorkspaceId context to its children
 */
export const LastWorkspaceIdProvider = (props: ParentProps<{}>) => {
	const [getter, setter] = makePersisted(createSignal<string | null>(null), {
		name: "lastWorkspaceId",
		storage: cookieStorage.withOptions({
			expires: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7), // 7 days
			path: "/",
			sameSite: "Strict",
		}),
	});

	return (
		<LastWorkspaceIdContext.Provider value={[getter, setter]}>
			{props.children}
		</LastWorkspaceIdContext.Provider>
	);
};

export function useLastWorkspaceId(): Signal<string | null> {
	const signal = useContext(LastWorkspaceIdContext);
	if (!signal) {
		throw new Error(
			"useLastWorkspaceId must be used within a LastWorkspaceIdProvider"
		);
	}

	return signal;
}

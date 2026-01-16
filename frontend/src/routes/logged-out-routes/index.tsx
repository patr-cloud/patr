import { Route, Navigate } from "@solidjs/router";
import Login from "~/pages/auth/login";
import SignUp from "~/pages/auth/sign-up";
import ForgotPassword from "~/pages/auth/forgot-password";
import { ParentProps } from "solid-js";
import { BgOnboard } from "~/components";
import ConfirmSignUp from "~/pages/auth/confirm-sign-up";
import { useAuthState } from "~/hooks";

export const AuthPageWrapper = (props: ParentProps<{}>) => {
	const [authState] = useAuthState();

	if (authState()?.type === "LoggedIn") {
		return <Navigate href="/" />;
	}

	return (
		<main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4 relative overflow-hidden">
			<BgOnboard />
			{props.children}
		</main>
	);
};

export default function LoggedOutRoutes() {
	return (
		<Route path="/" component={AuthPageWrapper}>
			<Route path="/login" component={Login} />
			<Route path="/sign-up" component={SignUp} />
			<Route path="/forgot-password" component={ForgotPassword} />
			<Route path="/confirm-signup" component={ConfirmSignUp} />
		</Route>
	);
}

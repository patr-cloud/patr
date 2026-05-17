import { createFileRoute, useNavigate, useRouter } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { onMount } from "solid-js";
import type { SocialLoginCallbackResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { cloudOnly } from "~/utils/env";
import { httpRequest } from "~/utils/http-request";

const GithubCallback = () => {
	const [, setAuthState] = useAuthState();
	const router = useRouter();
	const navigate = useNavigate();
	const toast = useToast();

	const search = Route.useSearch();

	const setLoggedIn = async (accessToken: string, refreshToken: string) => {
		const newAuth = {
			type: "LoggedIn" as const,
			accessToken,
			refreshToken,
		};
		setAuthState(newAuth);
		router.update({
			...router.options,
			context: {
				...router.options.context,
				auth: newAuth,
			},
		});
		await router.invalidate();
		navigate({ to: "/", replace: true });
	};

	onMount(async () => {
		const { code, state } = search();

		if (!code || !state) {
			toast("Invalid GitHub callback — missing code or state.", "error");
			navigate({ to: "/login", replace: true });
			return;
		}

		const resp = await httpRequest<SocialLoginCallbackResponse>("/api/auth/social-login/github/callback", {
			method: "POST",
			body: JSON.stringify({ code, state }),
		});

		if (!resp.ok) {
			toast(
				resp.data?.error === "socialLoginFailed"
					? "GitHub sign-in failed. Please try again."
					: "An unexpected error occurred. Please try again.",
				"error"
			);
			navigate({ to: "/login", replace: true });
			return;
		}

		const data = resp.data;

		switch (data.status) {
			case "loggedIn":
				await setLoggedIn(data.accessToken, data.refreshToken);
				break;

			case "setupRequired":
				navigate({
					to: "/sign-up/github",
					search: {
						setupToken: data.setupToken,
						username: data.prefilledUsername,
						firstName: data.prefilledFirstName,
						lastName: data.prefilledLastName,
						email: data.prefilledEmail,
					},
					replace: true,
				});
				break;
		}
	});

	return (
		<>
			<Title>GitHub Sign-In | Patr</Title>

			<div class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium text-center">
				<p class="text-gray-300 text-base">Signing in with GitHub...</p>
			</div>

			<div class="absolute bottom-6 left-0 right-0 text-center">
				<p class="text-gray-500 text-xs">&copy; {new Date().getFullYear()} Patr. All rights reserved.</p>
			</div>
		</>
	);
};

export const Route = createFileRoute("/_logged-out/login/github")(
	cloudOnly({
		validateSearch: (search: Record<string, unknown>): { code?: string; state?: string } => ({
			code: (search.code as string) || undefined,
			state: (search.state as string) || undefined,
		}),
		component: GithubCallback,
	}),
);

import { createFileRoute, useNavigate, useRouter } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, Match, onMount, Switch } from "solid-js";
import type { GithubOAuthCallbackResponse, GithubOAuthLinkRequest, GithubOAuthLinkResponse } from "~/bindings";
import { Button, useToast } from "~/components";
import { ButtonVariant } from "~/utils/color";
import { createAsyncAction, useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";

const GithubCallback = () => {
	const [, setAuthState] = useAuthState();
	const router = useRouter();
	const navigate = useNavigate();
	const toast = useToast();

	const search = Route.useSearch();

	// Three states: loading | linkRequired | error
	type PageState = "loading" | "linkRequired" | "error";
	const [pageState, setPageState] = createSignal<PageState>("loading");
	const [linkToken, setLinkToken] = createSignal<string>("");

	const setLoggedIn = (accessToken: string, refreshToken: string) => {
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
		navigate({ to: "/", replace: true });
	};

	// Exchange code + state with the backend on mount
	onMount(async () => {
		const { code, state } = search();

		if (!code || !state) {
			toast("Invalid GitHub callback — missing code or state.", "error");
			navigate({ to: "/login", replace: true });
			return;
		}

		const resp = await httpRequest<GithubOAuthCallbackResponse>("/api/auth/social-login/github/callback", {
			method: "POST",
			body: JSON.stringify({ code, state }),
		});

		if (!resp.ok) {
			toast(
				resp.data?.error === "githubOAuthFailed"
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
				setLoggedIn(data.accessToken!, data.refreshToken!);
				break;

			case "linkRequired":
				setLinkToken(data.linkToken!);
				setPageState("linkRequired");
				break;

			case "setupRequired":
				navigate({
					to: "/complete-github-signup",
					search: {
						setupToken: data.setupToken!,
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

	const { execute: confirmLink, isLoading: isLinking } = createAsyncAction(async () => {
		const body: GithubOAuthLinkRequest = { linkToken: linkToken() };
		const resp = await httpRequest<GithubOAuthLinkResponse>("/api/auth/social-login/github/link", {
			method: "POST",
			body: JSON.stringify(body),
		});

		if (resp.ok) {
			setLoggedIn(resp.data.accessToken, resp.data.refreshToken);
		} else {
			const msg =
				resp.data?.error === "githubOAuthFailed"
					? "The confirmation link has expired. Please sign in again."
					: "Failed to link your GitHub account. Please try again.";
			toast(msg, "error");
			navigate({ to: "/login", replace: true });
		}
	});

	return (
		<>
			<Title>GitHub Sign-In | Patr</Title>

			<Switch>
				{/* Loading state */}
				<Match when={pageState() === "loading"}>
					<div class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium text-center">
						<p class="text-gray-300 text-base">Signing in with GitHub...</p>
					</div>
				</Match>

				{/* Link-confirmation state */}
				<Match when={pageState() === "linkRequired"}>
					<div class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium">
						<div class="mb-8">
							<h1 class="font-bold text-2xl text-white mb-3">Link GitHub Account</h1>
							<p class="text-gray-400 text-sm leading-relaxed">
								A Patr account already exists with the email address associated with your GitHub
								account. Would you like to link your GitHub account to it?
							</p>
							<p class="text-gray-500 text-xs mt-3">
								You can then sign in with either your password or GitHub in the future.
							</p>
						</div>

						<div class="flex items-center gap-4">
							<Button
								variant={ButtonVariant.Contained}
								class="py-3 px-6 text-sm font-semibold flex-1"
								type="button"
								loading={isLinking}
								loadingContent={() => <span>Linking...</span>}
								onClick={() =>
									confirmLink().catch(() => {
										toast("An unexpected error occurred. Please try again.", "error");
									})
								}
							>
								Yes, Link My Account
							</Button>
							<Button
								variant={ButtonVariant.Outlined}
								class="py-3 px-6 text-sm font-semibold flex-1"
								type="button"
								onClick={() => navigate({ to: "/login", replace: true })}
							>
								Cancel
							</Button>
						</div>
					</div>
				</Match>
			</Switch>

			{/* Footer */}
			<div class="absolute bottom-6 left-0 right-0 text-center">
				<p class="text-gray-500 text-xs">&copy; {new Date().getFullYear()} Patr. All rights reserved.</p>
			</div>
		</>
	);
};

export const Route = createFileRoute("/_logged-out/github-callback")({
	validateSearch: (search: Record<string, unknown>): { code?: string; state?: string } => ({
		code: (search.code as string) || undefined,
		state: (search.state as string) || undefined,
	}),
	component: GithubCallback,
});

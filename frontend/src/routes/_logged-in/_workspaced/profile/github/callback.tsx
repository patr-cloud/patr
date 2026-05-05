import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { onMount } from "solid-js";
import { ConnectSocialLoginCallbackRequest } from "~/bindings";
import { LoadingSpinner, useToast } from "~/components";
import { httpRequest } from "~/utils/http-request";

const GithubConnectCallback = () => {
	const navigate = useNavigate();
	const toast = useToast();

	const search = Route.useSearch();

	onMount(async () => {
		const { code, state } = search();

		if (!code || !state) {
			toast("Invalid GitHub connect callback — missing code or state.", "error");
			navigate({ to: "/profile", replace: true });
			return;
		}

		const body: ConnectSocialLoginCallbackRequest = { code, state };
		const response = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/user/social-login/github/callback`, {
			method: "POST",
			body: JSON.stringify(body),
		});

		if (response.ok) {
			toast("GitHub account connected.", "success");
			navigate({ to: "/profile", replace: true });
			return;
		}

		const message =
			response.data?.error === "resourceAlreadyExists"
				? "This GitHub account is already linked to a different Patr user."
				: response.data?.error === "socialLoginFailed"
					? "The connect link has expired. Please try again."
					: "Failed to connect GitHub. Please try again.";
		toast(message, "error");
		navigate({ to: "/profile", replace: true });
	});

	return (
		<>
			<Title>Connecting GitHub | Patr</Title>
			<div class="flex h-screen w-full items-center justify-center gap-3 text-grey">
				<LoadingSpinner size={20} />
				<span class="text-sm">Connecting your GitHub account...</span>
			</div>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/profile/github/callback")({
	validateSearch: (search: Record<string, unknown>): { code?: string; state?: string } => ({
		code: (search.code as string) || undefined,
		state: (search.state as string) || undefined,
	}),
	component: GithubConnectCallback,
});

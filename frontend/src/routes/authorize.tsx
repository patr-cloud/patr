import { createFileRoute, redirect, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, For, Match, onMount, Show, Switch } from "solid-js";
import { Button, ButtonVariant } from "~/components";
import { httpRequest } from "~/utils/http-request";
import { GetConsentRequestResponse } from "~/bindings/GetConsentRequestResponse";
import { SubmitConsentRequest } from "~/bindings/SubmitConsentRequest";
import { SubmitConsentResponse } from "~/bindings/SubmitConsentResponse";

type Status = "loading" | "consent" | "deciding" | "expired" | "error";

/** What each identity scope lets the client see. */
const SCOPE_COPY: Record<string, { title: string; description: string }> = {
	openid: { title: "Sign you in", description: "Confirm who you are." },
	profile: { title: "Your name", description: "See your first and last name." },
	email: { title: "Your email address", description: "See the email address on your account." },
	offline_access: {
		title: "Stay signed in",
		description: "Keep access when you are not using the app, until you revoke it.",
	},
};

/**
 * The OAuth consent screen. At the root of the route tree, like
 * `accept-invite`, so a logged-out user can reach it and be sent to log in.
 */
const OAuthConsent = () => {
	const navigate = useNavigate();
	const search = Route.useSearch();

	const [status, setStatus] = createSignal<Status>("loading");
	const [consent, setConsent] = createSignal<GetConsentRequestResponse | null>(null);
	const [errorMessage, setErrorMessage] = createSignal("");

	onMount(() => {
		const requestId = search().requestId;

		if (!requestId) {
			setErrorMessage("This link is missing its authorization request.");
			setStatus("error");
			return;
		}

		void loadConsent(requestId);
	});

	const loadConsent = async (requestId: string) => {
		const response = await httpRequest<GetConsentRequestResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/auth/oauth/consent/${requestId}`,
			{ method: "GET" }
		);

		if (response.ok) {
			setConsent(response.data);
			setStatus("consent");
			return;
		}

		setStatus("expired");
	};

	const decide = async (approved: boolean) => {
		const requestId = search().requestId;
		if (!requestId) {
			return;
		}

		setStatus("deciding");

		const body: SubmitConsentRequest = { approved };
		const response = await httpRequest<SubmitConsentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/auth/oauth/consent/${requestId}`,
			{ method: "POST", body: JSON.stringify(body) }
		);

		if (!response.ok) {
			setStatus("expired");
			return;
		}

		window.location.replace(response.data.redirectUri);
	};

	return (
		<main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4">
			<Title>Authorize | Patr</Title>
			<div class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 border border-secondary-medium">
				<Switch>
					<Match when={status() === "loading"}>
						<p class="text-white text-center">Loading…</p>
					</Match>

					<Match when={status() === "consent" || status() === "deciding"}>
						<Show when={consent()}>
							{(details) => (
								<>
									<h1 class="text-white text-2xl font-bold mb-2 text-center">
										{details().clientName} wants to access your Patr account
									</h1>
									<p class="text-grey mb-8 text-center">
										You'll be returned to{" "}
										<span class="text-white">{details().redirectUriHost}</span>
										{details().previouslyApproved ? ", which you've connected before." : "."}
									</p>

									<ul class="flex flex-col gap-4 mb-8">
										<li>
											<p class="text-white">Act on your behalf</p>
											<p class="text-grey text-sm">
												Do anything in Patr with the same access you have.
											</p>
										</li>
										<For each={details().scopes}>
											{(scope) => (
												<li>
													<p class="text-white">{SCOPE_COPY[scope]?.title ?? scope}</p>
													<p class="text-grey text-sm">
														{SCOPE_COPY[scope]?.description ??
															"An additional permission this app has asked for."}
													</p>
												</li>
											)}
										</For>
									</ul>

									<div class="flex flex-col gap-3">
										<Button
											variant={ButtonVariant.Contained}
											loading={status() === "deciding"}
											loadingContent={() => <span>Authorizing…</span>}
											onClick={() => void decide(true)}
										>
											Authorize {details().clientName}
										</Button>
										<Button
											variant={ButtonVariant.Outlined}
											disabled={status() === "deciding"}
											onClick={() => void decide(false)}
										>
											Cancel
										</Button>
									</div>
								</>
							)}
						</Show>
					</Match>

					<Match when={status() === "expired"}>
						<h1 class="text-white text-2xl font-bold mb-4 text-center">Request expired</h1>
						<p class="text-grey mb-8 text-center">
							This authorization request is no longer valid. Start again from the app you were signing in
							to.
						</p>
						<Button variant={ButtonVariant.Outlined} onClick={() => navigate({ to: "/" })}>
							Go to dashboard
						</Button>
					</Match>

					<Match when={status() === "error"}>
						<h1 class="text-white text-2xl font-bold mb-4 text-center">Something's wrong with this link</h1>
						<p class="text-grey mb-8 text-center">{errorMessage()}</p>
						<Button variant={ButtonVariant.Outlined} onClick={() => navigate({ to: "/" })}>
							Go to dashboard
						</Button>
					</Match>
				</Switch>
			</div>
		</main>
	);
};

export const Route = createFileRoute("/authorize")({
	validateSearch: (search: Record<string, unknown>): { requestId?: string } => ({
		requestId: (search.requestId as string) || undefined,
	}),
	beforeLoad: ({ context, location }) => {
		if (!context.auth || context.auth.type === "LoggedOut") {
			throw redirect({
				to: "/login",
				search: { returnTo: `${location.pathname}${location.searchStr ?? ""}` },
			});
		}
	},
	component: OAuthConsent,
});

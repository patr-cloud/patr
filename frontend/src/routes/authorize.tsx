import { createFileRoute, redirect, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, For, Match, onMount, Show, Switch } from "solid-js";
import { Button, ButtonVariant } from "~/components";
import { httpRequest } from "~/utils/http-request";
import { GetConsentRequestResponse } from "~/bindings/GetConsentRequestResponse";
import { SubmitConsentRequest } from "~/bindings/SubmitConsentRequest";
import { SubmitConsentResponse } from "~/bindings/SubmitConsentResponse";

type Status = "loading" | "consent" | "deciding" | "expired" | "error";

/**
 * What each identity scope lets the client see. The access token itself acts
 * with the user's full permissions regardless of scope, which the screen
 * states separately so the list never reads as a limit.
 */
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
 * The OAuth consent screen.
 *
 * Sits at the root of the route tree, alongside `accept-invite`, rather than
 * under `_logged-in` or `_logged-out`. Both of those layouts guard in one
 * direction — one bounces anonymous users out, the other bounces authenticated
 * ones — and this page has to be reachable in both states: it shows consent to
 * a signed-in user, and sends anyone else to log in and come back.
 *
 * The client's own parameters never reach this page. `/authorize` validates
 * them, parks them server-side and hands the browser nothing but `requestId`,
 * so nothing here can influence which client is being approved or where the
 * user is sent afterwards.
 */
const OAuthConsent = () => {
	const navigate = useNavigate();
	const search = Route.useSearch();

	const [status, setStatus] = createSignal<Status>("loading");
	const [consent, setConsent] = createSignal<GetConsentRequestResponse | null>(null);
	const [errorMessage, setErrorMessage] = createSignal("");

	// Fetched on mount rather than in a loader: `onMount` does not run during
	// SSR, so the request goes out exactly once, from the browser, with a token
	// the client can refresh if it has just expired.
	onMount(() => {
		const requestId = search().requestId;

		if (search().error) {
			// `/authorize` could not trust the redirect URI enough to send the
			// failure back to the client, so it sent the user here instead.
			setErrorMessage(describeAuthorizeError(search().error));
			setStatus("error");
			return;
		}

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

		// `replace` rather than assigning `href`, so the back button does not
		// return to a consent request that has already been consumed.
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

/**
 * Turns the error code `/authorize` redirected here with into something a
 * user can act on. These only arrive when the request could not be sent back
 * to the client — an unknown client, or a redirect URI it never registered.
 */
const describeAuthorizeError = (code: string | undefined): string => {
	switch (code) {
		case "invalid_client":
			return "The app that sent you here isn't registered with Patr.";
		case "invalid_request":
			return "The app that sent you here made an invalid request.";
		default:
			return "The app that sent you here couldn't be verified.";
	}
};

export const Route = createFileRoute("/authorize")({
	validateSearch: (search: Record<string, unknown>): { requestId?: string; error?: string } => ({
		requestId: (search.requestId as string) || undefined,
		error: (search.error as string) || undefined,
	}),
	// Runs on the server with the auth state seeded from the cookie, so an
	// anonymous user is redirected before any HTML is streamed — no flash of a
	// consent card they aren't signed in to give.
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

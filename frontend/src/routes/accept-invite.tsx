import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, Match, onMount, Switch } from "solid-js";
import { Button, ButtonVariant, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { AcceptWorkspaceInviteRequest } from "~/bindings/AcceptWorkspaceInviteRequest";
import { AcceptWorkspaceInviteResponse } from "~/bindings/AcceptWorkspaceInviteResponse";
import { PreviewWorkspaceInviteRequest } from "~/bindings/PreviewWorkspaceInviteRequest";
import { PreviewWorkspaceInviteResponse } from "~/bindings/PreviewWorkspaceInviteResponse";

/**
 * Key under which a pending invite is stashed so it survives the login/sign-up
 * detour. Both this page (on return) and `login.tsx` (after a successful login)
 * read it.
 */
const PENDING_INVITE_KEY = "pendingWorkspaceInvite";

type Status = "loading" | "confirm" | "success" | "loggedOut" | "mismatch" | "expired" | "invalid";

const AcceptInvite = () => {
	const navigate = useNavigate();
	const toast = useToast();
	const [authState] = useAuthState();
	const [, setLastWorkspaceId] = useLastWorkspaceId();
	const search = Route.useSearch();

	const [status, setStatus] = createSignal<Status>("loading");
	const [workspaceName, setWorkspaceName] = createSignal("");
	const [isJoining, setIsJoining] = createSignal(false);
	const [invite, setInvite] = createSignal<{ inviteId: string; token: string } | null>(null);

	// The invite comes from the email link's search params, or from the stash if
	// the user is returning after logging in / signing up.
	const readInvite = (): { inviteId: string; token: string } | null => {
		const s = search();
		if (s.inviteId && s.token) return { inviteId: s.inviteId, token: s.token };
		try {
			const raw = sessionStorage.getItem(PENDING_INVITE_KEY);
			if (raw) return JSON.parse(raw) as { inviteId: string; token: string };
		} catch {
			// ignore malformed stash
		}
		return null;
	};

	// Fetch the workspace name so we can ask the user to confirm before joining.
	// Does not consume the invite.
	const loadPreview = async (inviteId: string, token: string) => {
		const body: PreviewWorkspaceInviteRequest = { inviteId, token };
		const response = await httpRequest<PreviewWorkspaceInviteResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/workspace-invite/preview`,
			{ method: "POST", body: JSON.stringify(body) }
		);

		if (response.ok) {
			setWorkspaceName(response.data.workspaceName);
			setStatus("confirm");
			return;
		}

		setStatus(response.data.error === "inviteExpired" ? "expired" : "invalid");
	};

	const accept = async () => {
		const current = invite();
		if (!current) return;

		setIsJoining(true);
		const body: AcceptWorkspaceInviteRequest = {
			inviteId: current.inviteId,
			token: current.token,
		};
		const response = await httpRequest<AcceptWorkspaceInviteResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/workspace-invite/accept`,
			{ method: "POST", body: JSON.stringify(body) }
		);
		setIsJoining(false);

		if (response.ok) {
			sessionStorage.removeItem(PENDING_INVITE_KEY);
			setLastWorkspaceId(response.data.id);
			setStatus("success");
			toast("You've joined the workspace", "success");
			navigate({ to: "/", replace: true });
			return;
		}

		switch (response.data.error) {
			case "inviteEmailMismatch":
				setStatus("mismatch");
				break;
			case "inviteExpired":
				setStatus("expired");
				break;
			default:
				setStatus("invalid");
				break;
		}
	};

	onMount(() => {
		const current = readInvite();
		if (!current) {
			setStatus("invalid");
			return;
		}
		setInvite(current);

		const auth = authState();
		if (auth?.type === "LoggedIn") {
			// Show a confirmation screen (with the workspace name) rather than
			// joining immediately.
			void loadPreview(current.inviteId, current.token);
		} else {
			// Stash the invite so the user returns to it after authenticating.
			try {
				sessionStorage.setItem(PENDING_INVITE_KEY, JSON.stringify(current));
			} catch {
				// ignore storage failures
			}
			setStatus("loggedOut");
		}
	});

	// A full navigation so the router context's auth state is rebuilt cleanly
	// (matches how the user dropdown logs out). The pending invite stays stashed
	// so login resumes the flow.
	const switchAccount = () => {
		window.location.href = "/login";
	};

	return (
		<main class="min-h-screen w-full bg-secondary flex items-center justify-center p-4">
			<Title>Accept Invite | Patr</Title>
			<div class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 border border-secondary-medium text-center">
				<Switch>
					<Match when={status() === "loading"}>
						<p class="text-white">Loading your invite…</p>
					</Match>
					<Match when={status() === "confirm"}>
						<h1 class="text-white text-2xl font-bold mb-4">
							You've been invited to join {workspaceName()}
						</h1>
						<p class="text-grey mb-8">
							Join <span class="text-white">{workspaceName()}</span> to access its resources with the
							roles you've been assigned.
						</p>
						<div class="flex flex-col gap-3">
							<Button
								variant={ButtonVariant.Contained}
								loading={isJoining()}
								loadingContent={() => <span>Joining…</span>}
								onClick={() => void accept()}
							>
								Join {workspaceName()}
							</Button>
							<Button variant={ButtonVariant.Outlined} onClick={() => navigate({ to: "/" })}>
								Maybe later
							</Button>
						</div>
					</Match>
					<Match when={status() === "success"}>
						<p class="text-white">You've joined the workspace! Redirecting…</p>
					</Match>
					<Match when={status() === "loggedOut"}>
						<h1 class="text-white text-2xl font-bold mb-4">You've been invited to a workspace</h1>
						<p class="text-grey mb-8">
							Sign in or create an account with the invited email address to accept.
						</p>
						<div class="flex flex-col gap-3">
							<Button variant={ButtonVariant.Contained} onClick={() => navigate({ to: "/login" })}>
								Log in
							</Button>
							<Button variant={ButtonVariant.Outlined} onClick={() => navigate({ to: "/sign-up" })}>
								Create an account
							</Button>
						</div>
					</Match>
					<Match when={status() === "mismatch"}>
						<h1 class="text-white text-2xl font-bold mb-4">Wrong account</h1>
						<p class="text-grey mb-8">
							This invite was sent to a different email address than the one you're signed in with. Log
							out and sign in with the invited email to accept.
						</p>
						<Button variant={ButtonVariant.Contained} onClick={switchAccount}>
							Log out &amp; switch account
						</Button>
					</Match>
					<Match when={status() === "expired"}>
						<h1 class="text-white text-2xl font-bold mb-4">Invite expired</h1>
						<p class="text-grey">This invite has expired. Please ask for a new invite.</p>
					</Match>
					<Match when={status() === "invalid"}>
						<h1 class="text-white text-2xl font-bold mb-4">Invalid invite</h1>
						<p class="text-grey">This invite link is invalid or has already been used.</p>
					</Match>
				</Switch>
			</div>
		</main>
	);
};

export const Route = createFileRoute("/accept-invite")({
	validateSearch: (search: Record<string, unknown>): { inviteId?: string; token?: string } => ({
		inviteId: (search.inviteId as string) || undefined,
		token: (search.token as string) || undefined,
	}),
	component: AcceptInvite,
});

import { createResource, For, Show } from "solid-js";
import { ConnectSocialLoginInitiateResponse, LinkedSocialLogin, ListSocialLoginsResponse } from "~/bindings";
import { Button, ButtonVariant, InputLabel, LoadingSpinner, useToast } from "~/components";
import { formatRelativeTime } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";

const fetchSocialLogins = async (): Promise<LinkedSocialLogin[]> => {
	const response = await httpRequest<ListSocialLoginsResponse>(
		`${import.meta.env.VITE_BASE_URL}/api/user/social-login`,
		{ method: "GET" }
	);
	if (!response.ok) {
		throw new Error(response.data.error ?? "Failed to load connected accounts");
	}
	return response.data.logins;
};

const ConnectedAccountsSection = () => {
	const toast = useToast();
	const [logins, { refetch }] = createResource(fetchSocialLogins);

	const githubLinked = () => logins()?.some((l) => l.provider === "github") ?? false;

	const handleConnectGithub = async () => {
		const response = await httpRequest<ConnectSocialLoginInitiateResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/social-login/github/connect`,
			{ method: "POST" }
		);
		if (response.ok) {
			window.location.href = response.data.authorizeUrl;
			return;
		}
		toast("Could not start the GitHub connect flow. Please try again.", "error");
	};

	const handleDisconnect = async (provider: LinkedSocialLogin["provider"]) => {
		const response = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/user/social-login/${provider}`, {
			method: "DELETE",
		});
		if (response.ok) {
			toast(`Disconnected ${provider}.`, "success");
			refetch();
			return;
		}
		toast(`Failed to disconnect ${provider}.`, "error");
	};

	return (
		<div class="flex gap-4 items-start w-full">
			<InputLabel parentClass="flex-1" label="Connected Accounts" />

			<div class="flex-11 flex flex-col gap-3">
				<Show
					when={!logins.loading}
					fallback={
						<div class="flex items-center gap-2 text-grey">
							<LoadingSpinner size={16} />
							<span class="text-sm">Loading...</span>
						</div>
					}
				>
					<Show
						when={(logins() ?? []).length > 0}
						fallback={<p class="text-sm text-grey">No third-party accounts connected.</p>}
					>
						<ul class="flex flex-col gap-2">
							<For each={logins()}>
								{(login) => (
									<li class="flex items-center justify-between rounded-xs border border-secondary-medium bg-secondary-light px-4 py-3">
										<div class="flex items-center gap-3">
											<Show when={login.provider === "github"}>
												<img
													src="/icons/github.svg"
													alt=""
													aria-hidden="true"
													height="20"
													width="20"
													class="invert"
												/>
											</Show>
											<div class="flex flex-col">
												<span class="text-sm text-white capitalize">{login.provider}</span>
												<span class="text-xs text-grey">
													Connected {formatRelativeTime(login.linkedAt)}
												</span>
											</div>
										</div>
										<Button
											variant={ButtonVariant.Outlined}
											type="button"
											class="py-1 px-3 text-xs"
											onClick={() => handleDisconnect(login.provider)}
										>
											Disconnect
										</Button>
									</li>
								)}
							</For>
						</ul>
					</Show>

					<Show when={!githubLinked() && !logins.loading}>
						<Button
							variant={ButtonVariant.Plain}
							class="self-start py-3 px-4 gap-3 rounded-xs bg-black! text-white! text-sm font-medium border border-white/25 enabled:hover:bg-[#1f1f1f]! enabled:hover:cursor-pointer transition-colors duration-200"
							type="button"
							onClick={handleConnectGithub}
						>
							<img
								src="/icons/github.svg"
								alt=""
								aria-hidden="true"
								height="20"
								width="20"
								class="invert"
							/>
							Connect GitHub
						</Button>
					</Show>
				</Show>
			</div>
		</div>
	);
};

export default ConnectedAccountsSection;

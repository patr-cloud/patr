import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { createSignal, Show } from "solid-js";
import {
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	ButtonVariant,
	Button,
} from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { AddDomainToWorkspaceRequest, AddDomainToWorkspaceResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";

// Check if input looks like a URL (has protocol, path, query, etc.)
function looksLikeUrl(input: string): boolean {
	const trimmed = input.trim();
	return /^https?:\/\//i.test(trimmed) || trimmed.includes("/") || trimmed.includes("?") || trimmed.includes("#");
}

// Extract hostname from URL-like input
function extractHostname(input: string): string {
	let trimmed = input.trim();

	// Remove protocol if present
	trimmed = trimmed.replace(/^https?:\/\//i, "");

	// Remove path, query, and fragment
	trimmed = trimmed.split(/[/?#]/)[0];

	return trimmed;
}

const CreateDomainPage = () => {
	const [domainInput, setDomainInput] = createSignal("");
	const [error, setError] = createSignal("");
	const [suggestedDomain, setSuggestedDomain] = createSignal("");
	const [isSubmitting, setIsSubmitting] = createSignal(false);
	let validationTimeout: number | undefined;

	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const navigate = useNavigate();

	const validateDomain = async (input: string) => {
		if (!input.trim()) {
			setError("");
			setSuggestedDomain("");
			return
		}

		const auth = authState();
		const wsId = workspaceId();

		if (!auth || auth.type !== "LoggedIn" || !wsId) {
			return
		}

		// First check if it looks like a URL
		if (looksLikeUrl(input)) {
			const hostname = extractHostname(input);
			setError("Please enter a domain without protocols, paths, or query strings");
			setSuggestedDomain(hostname);
			return
		}

		// Call the API to validate the domain
		try {
			const response = await httpRequest<{ valid: boolean }>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain/is-valid?domain=${encodeURIComponent(input)}`,
				{
					method: "GET",
				}
			)

			if (response.ok && response.data.valid) {
				setError("");
				setSuggestedDomain("");
			} else {
				// Domain is not valid (likely not a root domain or already exists)
				setError("Please enter a valid root domain (e.g., example.com, not subdomain.example.com)");
				setSuggestedDomain("");
			}
		} catch (err: any) {
			// Handle API errors
			if (err?.error === "NotRootDomain" || err?.error === "NotIcannDomain") {
				setError(
					err?.message || "Please enter a root domain (e.g., example.com instead of subdomain.example.com)"
				)
			} else if (err?.error === "ResourceAlreadyExists") {
				setError("This domain already exists in your workspace");
			} else {
				setError("Unable to validate domain. Please check your input.");
			}
			setSuggestedDomain("");
		}
	}

	const handleInputChange = (value: string) => {
		setDomainInput(value);

		// Clear previous timeout
		if (validationTimeout) {
			clearTimeout(validationTimeout);
		}

		// TODO Debounce the validation to avoid too many API calls
		// validationTimeout = setTimeout(() => {
		//   validateDomain(value);
		// }, 500) as unknown as number;
	}

	const handleSuggestionClick = () => {
		const suggested = suggestedDomain();
		setDomainInput(suggested);
		setError("");
		setSuggestedDomain("");
	}

	const onSubmit = async (e: SubmitEvent) => {
		e.preventDefault();

		const auth = authState();
		const wsId = workspaceId();

		if (!auth || auth.type !== "LoggedIn" || !wsId) {
			console.error("User is not logged in or workspace ID missing");
			return
		}

		const domain = domainInput().trim();

		if (!domain) {
			setError("Domain is required");
			return
		}

		// Quick check for URL-like input before submitting
		if (looksLikeUrl(domain)) {
			const hostname = extractHostname(domain);
			setError("Please enter a domain without protocols, paths, or query strings");
			setSuggestedDomain(hostname);
			return
		}

		// If there's already an error, don't submit
		if (error()) {
			return
		}

		setIsSubmitting(true);

		try {
			const requestBody: AddDomainToWorkspaceRequest = {
				domain: domain,
				nameserverType: "external",
			}

			const response = await httpRequest<AddDomainToWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain`,
				{
					method: "POST",
					body: JSON.stringify(requestBody),
				}
			)

			console.log("Domain added successfully:", response.data);
			navigate({ to: "/domains" });
		} catch (error) {
			console.error("Error adding domain:", error);
			setError("Failed to add domain. Please try again.");
		} finally {
			setIsSubmitting(false);
		}
	}

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Domains",
						url: "/domains",
					},
					{
						label: "Add",
					},
				]}
				subText="Configure custom domains to route traffic to your deployments."
			/>
			<PageContainerBody>
				<form onSubmit={onSubmit} class="space-y-6">
					<div class="bg-secondary-light p-6 rounded-xs border border-white/5">
						<div class="space-y-4">
							<div class="flex flex-col gap-2">
								<InputLabel parentClass="flex-2" for="domain-name" label="Domain Name" />
								<Input
									styleVariant="dark"
									id="domain-name"
									class="flex-10"
									name="domain-name"
									placeholder="example.com"
									type={InputType.Text}
									value={domainInput()}
									onInput={(e) => handleInputChange(e.currentTarget.value)}
									required
								/>
								<Show when={error()}>
									<p class="text-red-500 text-sm mt-1">{error()}</p>
								</Show>
								<Show when={suggestedDomain()}>
									<p class="text-gray-400 text-sm mt-1">
										Did you mean{" "}
										<button
											type="button"
											onClick={handleSuggestionClick}
											class="text-primary hover:underline font-medium"
										>
											{suggestedDomain()}
										</button>
										?
									</p>
								</Show>
							</div>

							<div class="bg-secondary-dark p-4 rounded border border-white/5">
								<h4 class="text-white text-sm font-semibold mb-2">Domain Requirements:</h4>
								<ul class="text-gray-400 text-sm space-y-1 list-disc list-inside">
									<li>✅ Enter only the base domain (e.g., example.com)</li>
									<li>❌ Do not include subdomains (e.g., www.example.com)</li>
									<li>❌ Do not include protocols (e.g., https://example.com)</li>
									<li>
										❌ Do not include paths or query parameters (e.g., example.com/path?query=1)
									</li>
									<li>⚠️ We currently don't support non-ASCII domains (e.g., èxámplê.com)</li>
								</ul>
							</div>
						</div>
					</div>

					<div class="w-full flex justify-end gap-4">
						<Button variant={ButtonVariant.Outlined} type="button" onClick={() => navigate({ to: "/domains" })}>
							Cancel
						</Button>
						<Button variant={ButtonVariant.Contained} type="submit" disabled={isSubmitting() || !!error()}>
							{isSubmitting() ? "Adding..." : "Add Domain"}
						</Button>
					</div>
				</form>
			</PageContainerBody>
		</PageContainer>
	)
};

export const Route = createFileRoute("/_logged-in/_workspaced/domains/new")({
	component: CreateDomainPage,
});

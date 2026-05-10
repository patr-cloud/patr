import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, Show } from "solid-js";
import {
	Alert,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	ButtonVariant,
	Button,
	Input,
	InputType,
	useToast,
	InputLabel,
} from "~/components";
import { createFormAction } from "~/hooks";
import { AddDomainToWorkspaceRequest, AddDomainToWorkspaceResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";

function looksLikeUrl(input: string): boolean {
	const trimmed = input.trim();
	return /^https?:\/\//i.test(trimmed) || trimmed.includes("/") || trimmed.includes("?") || trimmed.includes("#");
}

function extractHostname(input: string): string {
	let trimmed = input.trim();
	trimmed = trimmed.replace(/^https?:\/\//i, "");
	trimmed = trimmed.split(/[/?#]/)[0];
	return trimmed;
}

const CreateDomainPage = () => {
	const [domainInput, setDomainInput] = createSignal("");
	const [error, setError] = createSignal("");
	const [suggestedDomain, setSuggestedDomain] = createSignal("");
	const navigate = useNavigate();
	const toast = useToast();

	const handleSuggestionClick = () => {
		const suggested = suggestedDomain();
		setDomainInput(suggested);
		setError("");
		setSuggestedDomain("");
	};

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		const domain = domainInput().trim();

		if (!domain) {
			setError("Domain is required.");
			return;
		}

		if (looksLikeUrl(domain)) {
			const hostname = extractHostname(domain);
			setError("Enter a base domain only, without protocols, paths, or query strings.");
			setSuggestedDomain(hostname);
			return;
		}

		const requestBody: AddDomainToWorkspaceRequest = {
			domain,
		};

		const response = await httpRequest<AddDomainToWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			setError("Failed to add domain. Please try again.");
			return;
		}

		toast("Domain added successfully", "success");
		navigate({ to: "/domains" });
	});

	return (
		<>
			<Title>Add Domain | Patr</Title>
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
				<PageContainerBody class="flex flex-col">
					<form noValidate onSubmit={onSubmit} class="flex flex-col gap-8 w-full">
						<div class="flex flex-col gap-4 w-full">
							<div class="flex gap-8 items-start w-full">
								<InputLabel parentClass="flex-2 pt-2.5" for="domain-name" label="Domain Name" />
								<div class="flex-10 flex flex-col">
									<Input
										id="domain-name"
										name="domain-name"
										placeholder="example.com"
										type={InputType.Text}
										value={domainInput()}
										onInput={(e) => {
											setDomainInput(e.currentTarget.value);
											setError("");
											setSuggestedDomain("");
										}}
									/>
									<Show when={error()}>
										<div class="mt-1">
											<Alert message={error()} type="error" />
										</div>
									</Show>
									<Show when={suggestedDomain()}>
										<p class="text-grey text-sm mt-1">
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
									<div class="mt-3 bg-secondary-dark p-4 rounded border border-primary/40">
										<h4 class="text-white text-sm font-semibold mb-2">Domain Requirements:</h4>
										<ul class="text-grey text-sm space-y-1 list-disc list-inside">
											<li>✅ Enter only the base domain (e.g., example.com)</li>
											<li>❌ Do not include subdomains (e.g., www.example.com)</li>
											<li>❌ Do not include protocols (e.g., https://example.com)</li>
											<li>
												❌ Do not include paths or query parameters (e.g.,
												example.com/path?query=1)
											</li>
											<li>⚠️ We currently don't support non-ASCII domains (e.g., èxámplê.com)</li>
										</ul>
									</div>
								</div>
							</div>
						</div>

						<div class="w-full flex justify-end">
							<Button
								variant={ButtonVariant.Contained}
								type="submit"
								loading={isLoading}
								loadingContent={() => <span>Adding...</span>}
							>
								Add Domain
							</Button>
						</div>
					</form>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/domains/new")({
	component: CreateDomainPage,
});

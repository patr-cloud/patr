import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, Show } from "solid-js";
import { PageContainer, PageContainerBody, PageContainerHead } from "~/components";
import { CodeEntry } from "./-code-entry";
import { ConsentScreen } from "./-consent-screen";

type SearchParams = { code?: string };

const RunnerSetupPage = () => {
	const navigate = useNavigate();
	const search = Route.useSearch();
	const code = createMemo(() => search().code?.toUpperCase());

	return (
		<>
			<Title>Approve Runner | Patr</Title>
			<PageContainer>
				<PageContainerHead
					subText="A runner is asking to be added to this workspace"
					breadcrumbs={[{ label: "Runners", url: "/runners" }, { label: "Approve" }]}
				/>
				<PageContainerBody class="w-full">
					<div class="w-full">
						<Show
							when={code()}
							fallback={
								<CodeEntry onSubmit={(c) => navigate({ to: "/runner/setup", search: { code: c } })} />
							}
						>
							<ConsentScreen code={code()!} />
						</Show>
					</div>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/runner/setup/")({
	component: RunnerSetupPage,
	validateSearch: (raw: Record<string, unknown>): SearchParams => ({
		code: typeof raw.code === "string" ? raw.code : undefined,
	}),
});

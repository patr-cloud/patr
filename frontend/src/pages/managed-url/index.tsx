import { Input, InputDropdown, PageContainer, PageContainerBody, PageContainerHead } from "~/components";

const ManagedUrlPage = () => {
	return (
		<PageContainer>
			<PageContainerHead title="Managed URLs" subTitle="Create" />

			<PageContainerBody>
				<h1 class="text-md">Create Managed URL</h1>
				<div class="flex flex-col gap-2 items-start w-5/5 mt-4">
					<div class="flex items-center justify-center gap-2 w-full">
						<Input class="flex-2" placeholder="Sub-domain" />
						<span class="h-full">.</span>
						<Input class="flex-4" placeholder="Domain" />
						<span>/</span>
						<Input class="flex-2" placeholder="Path" />
						<p class="mx-2">Will point to</p>
						<InputDropdown
							onSelect={() => {}}
							options={[
								{
									label: "Deployments",
									value: "deployment",
								},
								{
									label: "Redirection",
									value: "redirection",
								},
								{
									label: "Proxy",
									value: "proxy",
								},
							]}
							class="flex-2 m-0"
							placeholder="Type"
						/>
						<Input class="flex-4" placeholder="Domain" />
					</div>
					<div class="flex items-center justify-center gap-2 w-full"></div>
				</div>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ManagedUrlPage;

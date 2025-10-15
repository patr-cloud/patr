import { createSignal } from "solid-js";
import {
  Input,
  InputLabel,
  InputType,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  InputDropdown,
} from "~/components";
import EnvInput from "./env-input";

const CreateDeploymentPage = () => {
  const [registry, setRegistry] = createSignal<string>("");

  return (
    <PageContainer>
      <PageContainerHead title="Deployment" subTitle="Create Deployment" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <div class="flex flex-col gap-6 items-start w-full">
          <h1 class="text-md">Create Deployment</h1>

          <div class="flex gap-8 items-center w-full">
            <InputLabel
              parentClass="flex-2"
              for="deployment-name"
              label="Name"
            />
            <Input
              class="flex-10"
              name="deployment-name"
              placeholder="Enter Token Name"
              type={InputType.Text}
            />
          </div>

          <div class="flex gap-8 items-center w-full">
            <InputLabel
              parentClass="flex-2"
              for="deployment-registry"
              label="Registry"
            />
            <InputDropdown
              options={[
                { value: "patr-registry", label: "Patr Registry" },
                { value: "docker-hub", label: "Docker Hub" },
              ]}
              value={registry()}
              onSelect={setRegistry}
              class="flex-10"
              name="deployment-registry"
              placeholder="Select Deployment Registry"
            />
          </div>

          <div class="flex gap-8 items-center w-full">
            <InputLabel parentClass="flex-2" label="Image Details" />

            <div class="flex items-center flex-10 gap-8">
              <Input
                class="flex-8"
                placeholder="Enter Deployment Image Name"
                type={InputType.Text}
              />
              <Input
                class="flex-4"
                placeholder="Enter Deployment Image Tag"
                type={InputType.Text}
              />
            </div>
          </div>

          <EnvInput onAdd={() => {}} onDelete={() => {}} />
        </div>
      </PageContainerBody>
    </PageContainer>
  );
};

export default CreateDeploymentPage;

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
import { FiChevronDown } from "solid-icons/fi";
import { Jsx } from "~/utils/func";
import { EnvironmentVariableValue } from "~/bindings";

const CreateDeploymentPage = () => {
  const [registry, setRegistry] = createSignal<string>("");
  const [envList, setEnvList] = createSignal<
    { key: string; value: EnvironmentVariableValue }[]
  >([{ key: "SAMPLE_KEY", value: "And Mand ka tola" }]);

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
              placeholder="Enter Deployment Name (e.g., andi-mandi-shandi)"
              type={InputType.Text}
            />
          </div>

          <div class="flex gap-8 items-center w-full">
            <InputLabel
              parentClass="flex-2"
              for="deployment-registry"
              label="Registry"
            />
            <div class="flex-10 flex items-center gap-4 w-full">
              <InputDropdown
                options={[
                  { value: "patr-registry", label: "Patr Registry" },
                  { value: "docker-hub", label: "Docker Hub" },
                ]}
                endIcon={Jsx(
                  <button>
                    <FiChevronDown size={16} />
                  </button>
                )}
                value={registry()}
                onSelect={setRegistry}
                class="flex-4"
                name="deployment-registry"
                placeholder="Select Registry"
              />

              <Input
                class="flex-6"
                placeholder="Image Name"
                type={InputType.Text}
              />

              <Input
                class="flex-2"
                placeholder="Image Tag"
                type={InputType.Text}
              />
            </div>
          </div>

          <EnvInput
            envList={envList}
            onAdd={(key, value) => {
              setEnvList((prev) => [...prev, { key, value }]);
            }}
            onDelete={(key) => {
              setEnvList((prev) => prev.filter((env) => env.key !== key));
            }}
          />
        </div>
      </PageContainerBody>
    </PageContainer>
  );
};

export default CreateDeploymentPage;

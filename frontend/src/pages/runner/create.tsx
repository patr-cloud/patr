import { createSignal } from "solid-js/types/server/reactive.js";
import {
  Input,
  InputLabel,
  InputType,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
} from "~/components";

const CreateRunnerPage = () => {
  const [name, setName] = createSignal<string>("");
  const onSubmit = (e: SubmitEvent) => {
    e.preventDefault();
  }

  return (
    <PageContainer>
      <PageContainerHead title="Runner" subTitle="New" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <div class="flex flex-col gap-6 items-start w-full">
          <h1 class="text-md">Create Runner</h1>

        <form
          onSubmit={onSubmit}
          class="flex w-full flex-col justify-between gap-8 h-full flex-1"
        >
          <div class="flex flex-col gap-6 items-start w-full">
            <h1 class="text-md">Create API Tokens</h1>

            <div class="flex gap-8 items-center w-full">
              <InputLabel
                parentClass="flex-2"
                for="token-name"
                label="Token Name"
              />
              <Input
                value={name()}
                onInput={(e) => {
                  setName(e.currentTarget.value);
                }}
                class="flex-10"
                name="token-name"
                placeholder="Enter Token Name"
                type={InputType.Text}
              />
            </div>
            </div></form>

        </div>
      </PageContainerBody>
    </PageContainer>
  );
};

export default CreateRunnerPage;

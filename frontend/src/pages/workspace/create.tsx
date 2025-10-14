import {
  Button,
  ButtonVariant,
  Input,
  InputLabel,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
} from "~/components";

const CreateWorkspace = () => {
  return (
    <PageContainer>
      <PageContainerHead title="Create Workspace" subTitle="subtitle" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <div class="flex gap-4 items-center">
          <InputLabel
            for="workspace-name"
            label="Workspace Name"
            parentClass="flex-2"
          />
          <Input
            name="workspace-name"
            placeholder="Enter Workspace Name. (Ex: Andi Mandi Shandi)"
            type="text"
            class="flex-10"
          />
        </div>

        <div class="flex justify-end w-full">
          <Button variant={ButtonVariant.Contained}>Create Workspace</Button>
        </div>
      </PageContainerBody>
    </PageContainer>
  );
};

export default CreateWorkspace;

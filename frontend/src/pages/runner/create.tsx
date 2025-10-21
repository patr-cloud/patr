import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
} from "~/components";

const CreateRunnerPage = () => {
  return (
    <PageContainer>
      <PageContainerHead title="Runner" subTitle="New" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <div class="flex flex-col gap-6 items-start w-full">
          <h1 class="text-md">Create Runner</h1>
        </div>
      </PageContainerBody>
    </PageContainer>
  );
};

export default CreateRunnerPage;

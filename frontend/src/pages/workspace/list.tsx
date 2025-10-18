import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
} from "~/components";

const ListWorkspaces = () => {
  return (
    <PageContainer>
      <PageContainerHead title="List Workspaces" subTitle="subtitle" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        workspace list page
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListWorkspaces;

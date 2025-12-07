import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  Table,
} from "~/components";

const ListApiTokens = () => {
  return (
    <PageContainer>
      <PageContainerHead title="API Tokens" subTitle="All Tokens" />
      <PageContainerBody class="flex flex-col gap-8">
        <Table
          column_grids={["flex-4", "flex-4", "flex-4"]}
          headings={["Token Name", "Expiry", "Created At"]}
          rows={[]}
        />
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListApiTokens;

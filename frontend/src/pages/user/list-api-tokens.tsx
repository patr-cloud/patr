import { A } from "@solidjs/router";
import {
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  Table,
} from "~/components";

const ListApiTokens = () => {
  return (
    <PageContainer>
      <PageContainerHead
        titleUrl="/profile"
        title="User"
        subTitle="API Tokens"
        actions={() => <A href="/profile/api-tokens/new">Create API Token</A>}
      />
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

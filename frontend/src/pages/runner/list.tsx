import {
  ContainerGrid,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  Table,
} from "~/components";
import RunnerCard from "./runner-card";

const ListRunnersPage = () => {
  return (
    <PageContainer>
      <PageContainerHead title="Runner" subTitle="List" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <Table
          column_grids={["flex-4", "flex-4", "flex-4"]}
          renderRow={(item) => {
            return (
              <>
                <td>{item.name}</td>
                <td>Status</td>
                <td>Created At</td>
              </>
            );
          }}
          headings={["Runner Name", "Status", "Created At"]}
          rows={[
            { name: "hi" },
            { name: "hello" },
            { name: "hey" },
            { name: "greetings" },
            { name: "salutations" },
          ]}
        />
        {/* <ContainerGrid
          renderCard={() => <RunnerCard />}
          items={[
          ]}
        /> */}
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListRunnersPage;

import {
  ContainerGrid,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
} from "~/components";
import RunnerCard from "./runner-card";

const ListRunnersPage = () => {
  return (
    <PageContainer>
      <PageContainerHead title="Runner" subTitle="List" />
      <PageContainerBody class="flex flex-col justify-between gap-8">
        <ContainerGrid
          renderCard={() => <RunnerCard />}
          items={["hi", "hello"]}
        />
      </PageContainerBody>
    </PageContainer>
  );
};

export default ListRunnersPage;

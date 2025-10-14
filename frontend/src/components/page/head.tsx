import { mergeProps } from "solid-js";

interface PageContainerHeadProps {
  title: string;
  subTitle: string;
}

const PageContainerHead = (rawProps: PageContainerHeadProps) => {
  const props = mergeProps({}, rawProps);

  return (
    <header class="h-full bg-secondary-light flex flex-col gap-2 rounded-t-xs p-xl py-lg">
      <div class="flex gap-4 items-center">
        <h1 class="text-2xl text-primary">Workspace</h1>
        <span class="text-xl text-white">&gt;</span>
        <h2 class="text-white text-md">Create Workspace</h2>
      </div>

      <p class="text-grey text-xs">
        Create a new workspace to organize your projects.
      </p>
    </header>
  );
};

export default PageContainerHead;

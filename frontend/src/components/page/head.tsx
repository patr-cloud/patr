import { JSX, mergeProps, ParentProps } from "solid-js";

interface PageContainerHeadProps {
  title: string;
  subTitle: JSX.Element | string;
  class?: string;
}

const PageContainerHead = (rawProps: ParentProps<PageContainerHeadProps>) => {
  const props = mergeProps(
    {
      class: "",
    },
    rawProps
  );

  return (
    <header
      class={`h-full bg-secondary-light flex gap-2 rounded-t-xs p-xl py-lg ${props.class}`}
    >
      <div class="flex flex-col gap-2 justify-start">
        <div class="flex gap-4 items-center">
          <h1 class="text-2xl text-primary">{props.title}</h1>
          <span class="text-xl text-white">&gt;</span>
          <h2 class="text-white text-md">{props.subTitle}</h2>
        </div>

        <p class="text-grey text-xs">
          Create a new workspace to organize your projects.
        </p>
      </div>

      {props.children}
    </header>
  );
};

export default PageContainerHead;

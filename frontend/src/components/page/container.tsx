import { mergeProps, ParentProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PageContainerProps {
  /** Additional Classes to add */
  class?: MaybeAccessor<string>;
}

const PageContainer = (rawProps: ParentProps<PageContainerProps>) => {
  const props = mergeProps({}, rawProps);

  return (
    <div
      class={`min-h-screen ${get(
        props.class
      )} bg-secondary p-sm pl-0 ml-sm flex flex-col`}
    >
      {props.children}
    </div>
  );
};

export default PageContainer;

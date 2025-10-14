import { mergeProps, ParentProps } from "solid-js";
import get from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PageContainerBodyProps {
  /**
   * Additional Classes for the body.
   */
  class?: MaybeAccessor<string>;
}

const PageContainerBody = (rawProps: ParentProps<PageContainerBodyProps>) => {
  const props = mergeProps(
    {
      class: "",
    },
    rawProps
  );

  return (
    <section
      class={`h-full bg-secondary-dark p-xl pl-[2.25rem] pt-[2.25rem] rounded-b-xs text-white flex-1 text-sm ${get(
        props.class
      )}`}
    >
      {props.children}
    </section>
  );
};

export default PageContainerBody;

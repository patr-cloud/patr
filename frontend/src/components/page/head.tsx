import { NavigateOptions, SearchParams } from "@solidjs/router";
import { JSX, mergeProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor, SetSearchParams } from "~/utils/types";

export interface HeadTabProps {
  /** Additional CSS classes for the tab container */
  class?: MaybeAccessor<string>;
  /** Additional CSS classes for the tab buttons */
  buttonClass?: MaybeAccessor<string>;
  /** Search Params */
  searchParams: Partial<SearchParams>;
  /** Tab */
  tab: MaybeAccessor<string>;
  /** Set Search Params */
  setSearchParams: (
    params: SetSearchParams,
    options?: Partial<NavigateOptions>
  ) => void;
  /** Tab Items */
  tabItems: Array<{
    /** Button Class */
    class?: MaybeAccessor<string>;
    /** The label of the tab */
    label: string;
    /** The value of the tab */
    value: string;
    /** OnClick handler for the tab */
    onClick?: (
      value: string,
      setSearchParams?: (
        params: SetSearchParams,
        options?: Partial<NavigateOptions>
      ) => void
    ) => void;
  }>;
}

const HeadTab = (rawProps: HeadTabProps) => {
  const props = mergeProps(
    {
      class: "",
      buttonClass: "",
    },
    rawProps
  );

  return (
    <div class={`w-full text-white flex gap-4 ${get(props.class)}`}>
      {props.tabItems.map((item) => (
        <button
          onClick={() => {
            if (item.onClick) {
              item.onClick(item.value, props.setSearchParams);
            }
          }}
          class={`pb-2 px-2 border-b-2 ${
            get(props.tab) === item.value ? "border-primary" : "border-none"
          } ${get(props.buttonClass)} ${get(item.class) || ""}`}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
};

interface PageContainerHeadProps {
  /** The title of the page head */
  title: string;
  /** Optional URL to redirect to when title is clicked */
  titleUrl?: string;
  /** The subtitle of the page head */
  subTitle: JSX.Element | string;
  /** Additional CSS classes for the header */
  class?: string;
  /** Actions to be displayed in the right side of header */
  actions?: () => JSX.Element;
  /** Bottom content of the header, e.g. switchable tabs */
  bottomContent?: () => JSX.Element;
}

const PageContainerHead = (rawProps: PageContainerHeadProps) => {
  const props = mergeProps(
    {
      class: "",
    },
    rawProps
  );

  return (
    <>
      <header
        class={`h-full bg-secondary-light flex justify-between items-center gap-2 rounded-t-xs p-xl py-lg ${props.class}`}
      >
        <div class="flex flex-col gap-2 justify-start">
          <div class="flex gap-4 items-center select-none">
            <h1
              class={`text-2xl text-primary ${
                props.titleUrl ? "cursor-pointer" : ""
              }`}
            >
              {props.titleUrl ? (
                <a href={props.titleUrl}>{props.title}</a>
              ) : (
                props.title
              )}
            </h1>
            <span class="text-xl text-white">&gt;</span>
            <h2 class="text-white text-md">{props.subTitle}</h2>
          </div>

          <p class="text-grey text-xs">
            Create a new workspace to organize your projects.
          </p>
        </div>

        <div>{props.actions?.()}</div>
      </header>
      {props.bottomContent && (
        <div class="bg-secondary-light px-xl">{props.bottomContent?.()}</div>
      )}
    </>
  );
};

export { HeadTab };
export default PageContainerHead;

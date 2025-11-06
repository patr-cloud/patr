import { mergeProps } from "solid-js";
import { Color } from "~/utils/color";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface StatusBadgeProps {
  /** Additional Classes for the badge.  */
  class?: MaybeAccessor<string>;
  /** The Text of the status Badge */
  text?: MaybeAccessor<string>;
  /** Status Color */
  color?: Color;
}

const StatusBadge = (rawProps: StatusBadgeProps) => {
  const props = mergeProps(
    {
      class: "",
    },
    rawProps
  );
  return (
    <span class="relative text-secondary cursor-default py-0.25 px-1.5 rounded-xl bg-info">
      {get(props.text)}
    </span>
  );
};

// {`relative ${get(props.class)}`}
export default StatusBadge;

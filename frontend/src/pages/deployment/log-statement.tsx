import { FiChevronRight } from "solid-icons/fi";
import { DeploymentLog } from "~/bindings";

interface LogStatementProps {
  log: DeploymentLog;
}

const LogStatement = (props: LogStatementProps) => {
  return (
    <div class="text-grey log-statement flex justify-start items-center w-full font-log hover:bg-grey/60">
      <FiChevronRight class="text-xs text-grey" />
      <time class="text-xxs pr-sm">{props.log.timestamp.toLocaleString()}</time>
      &nbsp;-&nbsp;
      <span class={`px-sm`}>{props.log.log}</span>
    </div>
  );
};

export default LogStatement;

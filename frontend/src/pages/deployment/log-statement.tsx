import { FiChevronRight } from "solid-icons/fi";
import { DeploymentLog } from "~/bindings";

interface LogStatementProps {
  log: DeploymentLog;
}

const LogStatement = (props: LogStatementProps) => {
  return (
    <div class="text-grey log-statement flex justify-start items-center w-full hover:bg-grey/60">
      <FiChevronRight class="text-xs text-grey" />
      <time class="text-xxs pr-xs font-log">
        {props.log.timestamp.toLocaleString()}
      </time>
      -<span class={`px-xs font-log`}>{props.log.log}</span>
    </div>
  );
};

export default LogStatement;

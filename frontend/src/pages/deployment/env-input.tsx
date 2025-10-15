import { createSignal } from "solid-js";
import {
  Button,
  ButtonVariant,
  Input,
  InputLabel,
  InputType,
} from "~/components";
import { MaybeAccessor } from "~/utils/types";

interface EnvInputProps {
  /** Additional Classes for the input.  */
  class?: MaybeAccessor<string>;
  /** On Add Value Handler */
  onAdd: (value: string) => void;
  /** On Remove Value Handler */
  onDelete: (value: string) => void;
}

const EnvInput = (props: EnvInputProps) => {
  const [envName, setEnvName] = createSignal<string>("");
  const [envValue, setEnvValue] = createSignal<string>("");

  return (
    <div class="flex gap-8 items-center w-full">
      <InputLabel parentClass="flex-2" label="Environment Variables" />

      <div class="flex items-center flex-10 gap-4">
        <Input
          class="flex-5"
          placeholder="Enter Env Name"
          type={InputType.Text}
          value={envName()}
          onInput={(e) => {
            setEnvName((e.target as HTMLInputElement).value);
          }}
        />
        <Input
          class="flex-6"
          placeholder="Enter Env Value"
          value={envValue()}
          type={InputType.Text}
          onInput={(e) => {
            setEnvValue((e.target as HTMLInputElement).value);
          }}
        />

        <Button variant={ButtonVariant.Contained} class="flex-1">
          Add
        </Button>
      </div>
    </div>
  );
};

export default EnvInput;

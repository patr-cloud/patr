import { FiPlus, FiTrash2 } from "solid-icons/fi";
import { createSignal } from "solid-js";
import { ExposedPortType } from "~/bindings";
import {
  Button,
  ButtonVariant,
  Input,
  InputDropdown,
  InputLabel,
} from "~/components";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PortInputProps {
  /** Additional Classes for the input.  */
  class?: MaybeAccessor<string>;
  /** On Add Port Handler */
  onAdd: (key: string, value: ExposedPortType) => void;
  /** On Remove Port Type Handler */
  onDelete: (key: string) => void;
  /** Port List */
  portList: MaybeAccessor<{ [key: string]: ExposedPortType }>;
}

const PortInput = (props: PortInputProps) => {
  const [portNumber, setPortNumber] = createSignal<string>("");
  const [portType, setPortType] = createSignal<ExposedPortType | undefined>(
    undefined
  );

  return (
    <div class={`${get(props.class)} flex gap-8 items-start w-full`}>
      <InputLabel parentClass="flex-2 pt-3" label="Exposed Ports" />

      <div class="flex flex-col flex-10 gap-4 w-full">
        {Object.entries(get(props.portList)).map(([key, value]) => (
          <div class="flex items-center flex-10 gap-4 w-full">
            <Input class="flex-6" disabled={true} value={key} />
            <Input class="flex-5" disabled={true} value={value} />

            <Button
              onClick={() => {
                props.onDelete(key);
              }}
              variant={ButtonVariant.Contained}
              class="flex-1 h-full flex items-center gap-2 bg-error"
            >
              <FiTrash2 size={16} />
            </Button>
          </div>
        ))}

        <div class="flex items-center flex-10 gap-4 w-full">
          <Input
            onInput={(e) => setPortNumber((e.target as HTMLInputElement).value)}
            class="flex-6"
            placeholder="Enter Port Number"
          />
          <InputDropdown
            placeholder="Select Port Type"
            onSelect={(value) => {
              setPortType(value as ExposedPortType);
            }}
            value={portType()}
            class="flex-5"
            options={[
              {
                value: "udp",
                label: "UDP",
              },
              {
                value: "tcp",
                label: "TCP",
              },
              {
                value: "http",
                label: "HTTP",
              },
            ]}
          />

          <Button
            variant={ButtonVariant.Contained}
            class="flex-1 h-full flex items-center gap-2"
            onClick={() => {
              const envVal = get(portType);
              if (!envVal) {
                return;
              }
              props.onAdd(portNumber(), envVal);
              setPortNumber("");
              setPortType(undefined);
            }}
          >
            <FiPlus size={16} />
          </Button>
        </div>
      </div>
    </div>
  );
};

export default PortInput;

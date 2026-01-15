import { FiPlus } from "solid-icons/fi";
import { Accessor, createSignal, For, Setter } from "solid-js";
import {
  Button,
  ButtonVariant,
  Input,
  InputEventT,
  InputLabel,
  InputType,
} from "~/components";
import { FileInput } from "~/components/input";
import { EventT } from "~/utils/types";

const MAX_SIZE_BYTES = 1 * 1024 * 1024; // 1 MB

export interface ConfigMountT {
  [key: string]: File;
}

interface ConfigMountProps {
  /** The current value */
  selectedFiles: Accessor<ConfigMountT>;
  setSelectedFiles: Setter<ConfigMountT>;
}

const ConfigMount = (props: ConfigMountProps) => {
  const [newFileName, setNewFileName] = createSignal<string>("");
  const [newFileContent, setNewFileContent] = createSignal<File | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  const handleChange = (e: Event & { currentTarget: HTMLInputElement }) => {
    // Handle file input change
    const files = e.currentTarget.files;

    if (files && files.length > 0) {
      const file = files[0];

      if (file && file.size <= MAX_SIZE_BYTES) setNewFileContent(file);
    }
  };

  const addConfig = (e: EventT<MouseEvent, HTMLButtonElement>) => {
    e.preventDefault();

    const fileContent = newFileContent();
    const fileName = newFileName().trim();

    if (!fileContent || !fileName) {
      setError("Please provide both a file name and select a file.");
      return;
    }

    if (fileContent.size > MAX_SIZE_BYTES) {
      setError("File size exceeds the maximum limit of 1 MB.");
      return;
    }

    props.setSelectedFiles({
      ...props.selectedFiles(),
      [fileName]: fileContent,
    });
  };

  return (
    <div class="flex flex-col gap-0 w-full">
      <div class="flex gap-8 items-center w-full">
        <InputLabel parentClass="flex-2" label="Config File" />
        <div class="flex-10 flex items-center gap-4 w-full">
          <Input
            type={InputType.Text}
            onInput={(e) => setNewFileName(e.currentTarget.value)}
            class="flex-6"
            id="deployment-config-filename"
            name="deployment-config-filename"
            placeholder="No file selected"
            value={newFileName()}
          />
          <FileInput
            id="deployment-config"
            name="deployment-config"
            onChange={handleChange}
            class="flex-5 file:bg-red-500"
            placeholder="Select Config File"
          />

          <Button
            type="button"
            variant={ButtonVariant.Contained}
            class="flex-1"
            onClick={addConfig}
          >
            <FiPlus size={16} />
          </Button>
        </div>
      </div>

      <For each={Object.entries(props.selectedFiles())}>
        {([fileName, file]) => (
          <div class="flex gap-8 items-center w-full mt-4">
            <div class="flex-2"></div>
            <section class="flex-10 flex items-center gap-4 w-full">
              <Input
                type={InputType.Text}
                onInput={(e) => {
                  const newFileName = e.currentTarget.value;

                  const currentFiles = props.selectedFiles();
                  const newFiles = { ...currentFiles };

                  // Remove old key and add new key
                  delete newFiles[fileName];
                  newFiles[newFileName] = file;

                  props.setSelectedFiles(newFiles);
                }}
                class="flex-6"
                name="deployment-config-filename"
                placeholder="No file selected"
                value={fileName}
              />
              <Input
                type={InputType.Text}
                value={file.name}
                class="flex-5"
                name="deployment-config"
                placeholder="Select Config File"
                disabled
              />
            </section>
          </div>
        )}
      </For>
    </div>
  );
};

export default ConfigMount;

import { createSignal } from "solid-js";
import Modal, { ModalContainer } from "~/components/modal";
import Input, { InputType } from "~/components/input";
import Button from "~/components/button";
import { ButtonVariant } from "~/utils/color";

const DeleteModal = (props: {
  onClickDelete: (e: MouseEvent & { currentTarget: HTMLButtonElement }) => void;
  resourceName: string;
  title: string;
}) => {
  const [resourceNameInput, setResourceNameInput] = createSignal("");

  return (
    <Modal
      renderModalContent={(close) => (
        <ModalContainer closeFn={() => close(false)} class="w-full">
          <form class="w-full">
            <h2 class="text-md text-primary font-semibold mb-4">{props.title}</h2>
            <p class="mb-4 text-sm text-white">
              This action cannot be undone. To Confirm, type &nbsp;
              <b>"{props.resourceName}"</b> below.
            </p>

            <Input
              type={InputType.Text}
              styleVariant="medium"
              class="mb-3"
              value={resourceNameInput()}
              onInput={(e) =>
                setResourceNameInput((e.target as HTMLInputElement).value)
              }
            />
            <div class="flex w-full justify-end items-center">
              <Button
                variant={ButtonVariant.Contained}
                type="submit"
                onClick={props.onClickDelete}
                disabled={resourceNameInput() !== props.resourceName}
              >
                DELETE
              </Button>
            </div>
          </form>
        </ModalContainer>
      )}
      renderTrigger={(open) => (
        <Button onClick={() => open(true)} variant={ButtonVariant.Contained}>
          DELETE
        </Button>
      )}
    />
  );
};

export default DeleteModal;

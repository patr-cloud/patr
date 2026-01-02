import { FiX } from "solid-icons/fi";
import { JSX, ParentProps, createSignal, mergeProps } from "solid-js";
import { Portal } from "solid-js/web";

interface ModalProps {
  renderTrigger: (setClose: (prev: boolean) => void) => JSX.Element;
  renderModalContent: (setClose: (prev: boolean) => void) => JSX.Element;
}

interface ModalContainerProps {
  style?: JSX.CSSProperties;
  class?: string;
  width?: string;
  height?: string;
  closeFn: (prev: boolean) => void;
}

const ModalContainer = (rawProps: ParentProps<ModalContainerProps>) => {
  const props = mergeProps(
    {
      style: {},
      class: "",
    },
    rawProps
  );

  return (
    <div
      style={{
        width: props.width || "auto",
        height: props.height || "auto",
        ...props.style,
      }}
      class={`relative bg-secondary-light rounded-xs p-6 w-full mx-4 min-h-[200px] min-w-[300px] shadow-lg ${props.class}`}
    >
      <button
        onClick={() => props.closeFn(false)}
        class="absolute w-5 h-5 top-4 right-4 bg-primary rounded-full cursor-pointer flex justify-center items-center hover:bg-primary/80 transition"
      >
        <FiX size="16" />
      </button>
      {props.children}
    </div>
  );
};

const Modal = ({ renderTrigger, renderModalContent }: ModalProps) => {
  const [isOpen, setIsOpen] = createSignal(false);

  return (
    <>
      {renderTrigger(setIsOpen)}
      {isOpen() && (
        <Portal>
          <div class="w-full min-h-screen fixed top-0 left-0 bg-black/50 flex justify-center items-center z-50 backdrop-blur-sm">
            {renderModalContent(setIsOpen)}
          </div>
        </Portal>
      )}
    </>
  );
};

export { ModalContainer };
export default Modal;

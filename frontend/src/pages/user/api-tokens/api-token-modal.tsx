import { Accessor, Setter } from "solid-js";
import { FiCopy } from "solid-icons/fi";
import Modal, { ModalContainer } from "~/components/modal";
import { useToast } from "~/components/toast";

const ApiTokenModal = (props: {
    isOpen: Accessor<boolean>;
    setIsOpen: Setter<boolean>;
    token: Accessor<string>;
    onClose?: () => void;
}) => {
    const toast = useToast();

    const handleClose = () => {
        props.setIsOpen(false);
        props.onClose?.();
    };

    return (
        <Modal
            isOpen={props.isOpen}
            renderTrigger={() => <></>}
            renderModalContent={() => (
                <ModalContainer
                    closeFn={handleClose}
                    class="w-200 p-6 bg-secondary-medium rounded shadow-lg"
                >
                    <h2 class="text-md mb-4 text-primary">API Token Created Successfully</h2>
                    <p class="mb-3 text-sm text-white">
                        Please copy your API token now. You won't be able to see it again!
                    </p>
                    <div class="bg-secondary-light text-white text-sm px-4 py-2 rounded-xs flex items-center justify-between">
                        <pre class="break-all">{props.token()}</pre>

                        <button
                            class="p-2 rounded-xs flex items-center hover:bg-secondary-dark/80 transition"
                            onClick={() => {
                                navigator.clipboard.writeText(props.token());
                                toast("API Token copied to clipboard", "success");
                            }}
                        >
                            <FiCopy size={16} />
                        </button>
                    </div>
                </ModalContainer>
            )}
        />
    );
};

export default ApiTokenModal;

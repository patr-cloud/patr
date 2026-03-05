import { Accessor, Setter } from "solid-js";
import Modal, { ModalContainer } from "~/components/modal";
import { useToast } from "~/components/toast";
import CopyableField from "~/components/copyable-field";

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
                    <CopyableField
                        value={props.token()}
                        onCopy={() => toast("API Token copied to clipboard", "success")}
                    />
                </ModalContainer>
            )}
        />
    );
};

export default ApiTokenModal;

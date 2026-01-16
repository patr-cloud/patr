import { Accessor, createSignal, Setter } from "solid-js";
import Modal, { ModalContainer } from "~/components/modal";
import Input, { InputType } from "~/components/input";
import Button from "~/components/button";
import { ButtonVariant, Color } from "~/utils/color";

const DeleteModal = (props: {
	onClickDelete: (e: MouseEvent & { currentTarget: HTMLButtonElement }) => void;
	resourceName: string;
	title: string;
	isOpen: Accessor<boolean>;
	setIsOpen: Setter<boolean>;
}) => {
	const [resourceNameInput, setResourceNameInput] = createSignal("");
	const [internalIsOpen, internalSetIsOpen] = createSignal(false);

	// Use external state if provided, otherwise use internal state
	const isOpen = props.isOpen || internalIsOpen;
	const setIsOpen = props.setIsOpen || internalSetIsOpen;
	return (
		<Modal
			isOpen={isOpen}
			setIsOpen={setIsOpen}
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
							onInput={(e) => setResourceNameInput((e.target as HTMLInputElement).value)}
						/>
						<div class="flex w-full justify-end items-center">
							<Button
								variant={ButtonVariant.Outlined}
								type="submit"
								onClick={props.onClickDelete}
								disabled={resourceNameInput() !== props.resourceName}
								color={Color.Error}
							>
								DELETE
							</Button>
						</div>
					</form>
				</ModalContainer>
			)}
			renderTrigger={(open) => (
				<Button onClick={() => open(true)} variant={ButtonVariant.Outlined} color={Color.Error}>
					DELETE
				</Button>
			)}
		/>
	);
};

export default DeleteModal;

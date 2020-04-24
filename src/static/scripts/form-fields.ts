export default class FormFields {
    private element: HTMLFormElement
    constructor(formElem: HTMLFormElement) {
        this.element = formElem;
    }

    toObject() {
        const obj:{[key: string]: any} = {}
        for (let i=0; i < this.element.elements.length; i++) {
            const inputElem = this.element.elements[i] as HTMLInputElement;
            const inputType = inputElem.getAttribute('type');
            const inputName = inputElem.getAttribute('name');
            if (inputType == 'text' || inputType == 'password') {
                obj[inputName] = inputElem.value;
            } else if (inputType == 'checkbox') {
                obj[inputName] = inputElem.checked;
            }
        }
        return obj;
    }
}
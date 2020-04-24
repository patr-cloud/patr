import axios from 'axios';
import FormFields from './form-fields';

const form = <HTMLFormElement>document.getElementsByClassName('form-signin')[0];
const loginFields = new FormFields(form);

async function login() {
    const result = (await axios.post('/oauth/login', loginFields.toObject())).data;

    if (result.success) {
        window.location = result.redirect;
    } else {
        alert(result.message);
    }
}

window.addEventListener('load', () => {
    form.addEventListener('submit', (e) => {
        e.preventDefault();
        login();
    })
});

import { ServerConnection } from './server';
import './style.css'

document.getElementById("connect")?.addEventListener("click", async () => {
    let server = new ServerConnection();
});

/* @refresh reload */
import { render } from "solid-js/web";
import App from "./App";
import "./index.css";
import { registerServiceWorker } from "./register-service-worker";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Root element not found");
}

registerServiceWorker();
render(() => <App />, root);

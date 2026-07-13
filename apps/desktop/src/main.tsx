import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@cancan/ui/foundation.css";

import { App } from "./app";

const root = document.getElementById("root");

if (!root) {
  throw new Error("CanCan root element is missing");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

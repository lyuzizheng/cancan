import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";

import { App, routeFromPath } from "./App";

const container = document.getElementById("root");

if (!container) {
  throw new Error("CanCan website root element is missing");
}

const app = (
  <StrictMode>
    <App route={routeFromPath(window.location.pathname)} />
  </StrictMode>
);

// Pre-rendered pages hydrate; the dev server renders from scratch.
if (container.hasChildNodes()) {
  hydrateRoot(container, app);
} else {
  createRoot(container).render(app);
}

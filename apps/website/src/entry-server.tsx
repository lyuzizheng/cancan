import { renderToString } from "react-dom/server";

import { App, type Route } from "./App";

export function render(route: Route): string {
  return renderToString(<App route={route} />);
}

import { renderToStaticMarkup } from "react-dom/server";

import { App, type Route } from "./App";

export function render(route: Route): string {
  return renderToStaticMarkup(<App route={route} />);
}

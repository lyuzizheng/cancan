import { Layout, type Route } from "./components/Layout";
import { DocsPage } from "./pages/DocsPage";
import { DownloadPage } from "./pages/DownloadPage";
import { HomePage } from "./pages/HomePage";
import { NotFoundPage } from "./pages/NotFoundPage";
import { PrivacyPage } from "./pages/PrivacyPage";
import { SecurityPage } from "./pages/SecurityPage";
import "./styles/site.css";

export type { Route };

const PAGES = {
  docs: DocsPage,
  download: DownloadPage,
  home: HomePage,
  "not-found": NotFoundPage,
  privacy: PrivacyPage,
  security: SecurityPage,
} as const;

export function App({ route }: { route: Route }) {
  const Page = PAGES[route];
  return (
    <Layout route={route}>
      <Page />
    </Layout>
  );
}

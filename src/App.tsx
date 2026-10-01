import { useState } from "react";

import { Sidebar, type PageId } from "@/components/layout/Sidebar";
import { isMockMode } from "@/lib/api/client";
import { IndexingPage } from "@/pages/IndexingPage";
import { LibraryPage } from "@/pages/LibraryPage";
import { PrivacyPage } from "@/pages/PrivacyPage";
import { SearchPage } from "@/pages/SearchPage";

const PAGES: Record<PageId, () => React.JSX.Element> = {
  search: SearchPage,
  library: LibraryPage,
  indexing: IndexingPage,
  privacy: PrivacyPage,
};

export default function App() {
  const [page, setPage] = useState<PageId>("search");
  const Page = PAGES[page];

  return (
    <div className="flex h-full flex-col">
      {isMockMode && (
        <div className="bg-amber-500 px-4 py-1 text-center text-xs font-medium text-black">
          Mock mode — showing fake development data, not your files.
        </div>
      )}
      <div className="flex min-h-0 flex-1">
        <Sidebar current={page} onNavigate={setPage} />
        <main className="min-w-0 flex-1 overflow-y-auto px-10 py-8">
          <Page />
        </main>
      </div>
    </div>
  );
}

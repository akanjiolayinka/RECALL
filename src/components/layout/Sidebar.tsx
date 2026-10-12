import { Activity, Library, Search, ShieldCheck, type LucideIcon } from "lucide-react";

import { SEARCH_SHORTCUT_LABEL } from "@/lib/shortcuts";
import { cn } from "@/lib/utils";

import { BackendStatus } from "./BackendStatus";

export type PageId = "search" | "library" | "indexing" | "privacy";

const NAV_ITEMS: { id: PageId; label: string; icon: LucideIcon }[] = [
  { id: "search", label: "Search", icon: Search },
  { id: "library", label: "Library", icon: Library },
  { id: "indexing", label: "Indexing", icon: Activity },
  { id: "privacy", label: "Privacy", icon: ShieldCheck },
];

interface SidebarProps {
  current: PageId;
  onNavigate: (page: PageId) => void;
}

export function Sidebar({ current, onNavigate }: SidebarProps) {
  return (
    <aside className="flex w-14 shrink-0 flex-col border-r bg-sidebar md:w-56">
      <div className="hidden px-5 pt-6 pb-4 md:block">
        <p className="text-lg font-semibold tracking-tight">Recall</p>
        <p className="text-xs text-muted-foreground">Your private AI memory</p>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-2 pt-4 md:px-3 md:pt-0" aria-label="Main">
        {NAV_ITEMS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => onNavigate(id)}
            aria-current={current === id ? "page" : undefined}
            title={label}
            className={cn(
              "flex items-center justify-center gap-3 rounded-md px-3 py-2 md:justify-start text-sm font-medium text-muted-foreground transition-colors outline-none hover:bg-accent hover:text-accent-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50",
              current === id && "bg-accent text-accent-foreground",
            )}
          >
            <Icon className="size-4 shrink-0" aria-hidden />
            <span className="sr-only md:not-sr-only">{label}</span>
            {id === "search" && (
              <kbd className="ml-auto hidden rounded md:inline border bg-background px-1.5 font-sans text-[10px] text-muted-foreground">
                {SEARCH_SHORTCUT_LABEL}
              </kbd>
            )}
          </button>
        ))}
      </nav>

      <div className="border-t px-5 py-4 max-md:flex max-md:justify-center max-md:px-0">
        <BackendStatus />
      </div>
    </aside>
  );
}

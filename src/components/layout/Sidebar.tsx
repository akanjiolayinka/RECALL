import { Activity, Library, Search, ShieldCheck, type LucideIcon } from "lucide-react";

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
    <aside className="flex w-56 shrink-0 flex-col border-r bg-sidebar">
      <div className="px-5 pt-6 pb-4">
        <p className="text-lg font-semibold tracking-tight">Recall</p>
        <p className="text-xs text-muted-foreground">Your private AI memory</p>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-3" aria-label="Main">
        {NAV_ITEMS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => onNavigate(id)}
            aria-current={current === id ? "page" : undefined}
            className={cn(
              "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors outline-none hover:bg-accent hover:text-accent-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50",
              current === id && "bg-accent text-accent-foreground",
            )}
          >
            <Icon className="size-4" aria-hidden />
            {label}
          </button>
        ))}
      </nav>

      <div className="border-t px-5 py-4">
        <BackendStatus />
      </div>
    </aside>
  );
}

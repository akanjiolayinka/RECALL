import { FileImage, FileText, FileType, type LucideIcon } from "lucide-react";

import type { FileKind } from "@/lib/api/client";

const ICONS: Record<FileKind, LucideIcon> = {
  pdf: FileType,
  docx: FileText,
  text: FileText,
  markdown: FileText,
  image: FileImage,
};

export const FILE_KIND_LABELS: Record<FileKind, string> = {
  pdf: "PDF",
  docx: "Word",
  text: "Text",
  markdown: "Markdown",
  image: "Image",
};

export function FileKindIcon({ kind, className }: { kind: FileKind; className?: string }) {
  const Icon = ICONS[kind];
  return <Icon className={className} aria-hidden />;
}

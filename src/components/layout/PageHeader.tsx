import type { ReactNode } from "react";

interface PageHeaderProps {
  title: string;
  description?: ReactNode;
}

export function PageHeader({ title, description }: PageHeaderProps) {
  return (
    <header className="space-y-1">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      {description && <p className="text-sm text-muted-foreground">{description}</p>}
    </header>
  );
}

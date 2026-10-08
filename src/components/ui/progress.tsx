import { cn } from "@/lib/utils";

interface ProgressProps {
  /** 0–100, or null for "working, amount unknown". */
  value: number | null;
  className?: string;
  label: string;
}

/** Horizontal progress bar. */
export function Progress({ value, className, label }: ProgressProps) {
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value ?? undefined}
      className={cn("h-1.5 w-full overflow-hidden rounded-full bg-muted", className)}
    >
      <div
        className={cn(
          "h-full rounded-full bg-primary transition-[width] duration-200",
          value === null && "w-1/3 animate-pulse",
        )}
        style={value === null ? undefined : { width: `${Math.min(100, Math.max(0, value))}%` }}
      />
    </div>
  );
}

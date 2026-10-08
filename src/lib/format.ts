/** Display helpers for numbers, sizes and dates. */

const numberFormat = new Intl.NumberFormat();

/** 1028 → "1,028" (using the user's locale). */
export function formatCount(value: number): string {
  return numberFormat.format(value);
}

/** 2400000 → "2.3 MB". */
export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${unit === 0 ? value : value.toFixed(1)} ${units[unit]}`;
}

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

/** Milliseconds since epoch → "Oct 8, 2026", or "—" when unknown. */
export function formatDate(millis: number | null): string {
  return millis === null ? "—" : dateFormat.format(new Date(millis));
}

/** "1 file" / "2 files". */
export function plural(count: number, singular: string, pluralForm = `${singular}s`): string {
  return `${formatCount(count)} ${count === 1 ? singular : pluralForm}`;
}

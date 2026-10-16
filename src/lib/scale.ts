// GUI scale: each page has its own level, 1 (smallest)
// to 5, and 3 is the normal size. Applied as CSS zoom on the page.
import type { ScalePage, Settings } from "./api";

const ZOOM = [0.8, 0.9, 1, 1.1, 1.2];

export const SCALE_LEVELS = [1, 2, 3, 4, 5];

export function scaleLevel(settings: Settings | null, page: ScalePage): number {
  const level = settings?.uiScale?.[page] ?? 3;
  return Math.min(5, Math.max(1, Math.round(level)));
}

export const zoomFor = (level: number) => ZOOM[Math.min(5, Math.max(1, level)) - 1];

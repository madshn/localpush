export function visibleRefetchInterval(ms: number): number | false {
  if (typeof document === "undefined") return ms;
  return document.visibilityState === "hidden" ? false : ms;
}


export interface Toast {
  id: number;
  text: string;
  kind: "info" | "error";
}

let nextId = 1;

export const toasts = $state<Toast[]>([]);

export function toast(text: string, kind: Toast["kind"] = "info", ms = 4000) {
  const id = nextId++;
  toasts.push({ id, text, kind });
  setTimeout(() => {
    const i = toasts.findIndex((t) => t.id === id);
    if (i >= 0) toasts.splice(i, 1);
  }, ms);
}

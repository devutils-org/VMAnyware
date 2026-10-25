// Application keyboard shortcuts. The modifier is ⌘ on macOS and Ctrl+Alt elsewhere,
// so shortcuts never collide with keys typed into a terminal or a field.
export type ShortcutId =
  | "nav1" | "nav2" | "nav3" | "nav4" | "nav5" | "nav6"
  | "new" | "refresh" | "connect" | "fullWindow" | "prevSession" | "nextSession" | "closeSession" | "back" | "help";

type Spec = { key: string; shift?: boolean; label: string; description: string };
export const shortcuts: Record<ShortcutId, Spec> = {
  nav1: { key: "1", label: "1", description: "Environments" },
  nav2: { key: "2", label: "2", description: "New environment" },
  nav3: { key: "3", label: "3", description: "Sessions" },
  nav4: { key: "4", label: "4", description: "Activity" },
  nav5: { key: "5", label: "5", description: "Keychain" },
  nav6: { key: "6", label: "6", description: "Settings" },
  new: { key: "n", label: "N", description: "New environment" },
  refresh: { key: "r", label: "R", description: "Refresh the current view" },
  connect: { key: "t", label: "T", description: "New connection (machine console or Sessions)" },
  fullWindow: { key: "f", shift: true, label: "⇧F", description: "Toggle full-window terminal" },
  prevSession: { key: "[", shift: true, label: "⇧[", description: "Previous session" },
  nextSession: { key: "]", shift: true, label: "⇧]", description: "Next session" },
  closeSession: { key: "w", shift: true, label: "⇧W", description: "Close the active session" },
  back: { key: "[", label: "[", description: "Back to the list" },
  help: { key: "/", label: "/", description: "Show keyboard shortcuts" },
};

export const isMac = () => typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
export const modifierLabel = () => (isMac() ? "⌘" : "Ctrl+Alt+");
export const shortcutLabel = (id: ShortcutId) => modifierLabel() + shortcuts[id].label;

// Physical keys for punctuation, so ⇧ does not change which key matched.
const codes: Record<string, string> = { "[": "BracketLeft", "]": "BracketRight", "/": "Slash" };

/** The shortcut an event triggers, if any. */
export function matchShortcut(event: { key: string; code: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }, mac = isMac()): ShortcutId | null {
  const modifier = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && event.altKey && !event.metaKey;
  if (!modifier) return null;
  for (const [id, spec] of Object.entries(shortcuts) as [ShortcutId, Spec][]) {
    if (!!spec.shift !== event.shiftKey) continue;
    const code = codes[spec.key];
    if (code ? event.code === code : event.key.toLowerCase() === spec.key || event.code === `Digit${spec.key}` || event.code === `Key${spec.key.toUpperCase()}`) return id;
  }
  return null;
}

// Small inline stroke icons; no icon font or runtime dependency.
const paths: Record<string, string> = {
  servers: "M4 5h16v5H4zM4 14h16v5H4zM8 7.5h.01M8 16.5h.01",
  plus: "M12 5v14M5 12h14",
  terminal: "M4 5h16v14H4zM8 10l3 2-3 2M13 15h3",
  activity: "M3 12h4l3-7 4 14 3-7h4",
  settings: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM19 12l2-1-1-3-2 .2-1.5-1.5L16.7 4l-3-1-1 2h-1.4l-1-2-3 1 .2 2.7L6 8.2 4 8l-1 3 2 1v1.4l-2 1 1 3 2-.2 1.5 1.5L7.3 20l3 1 1-2h1.4l1 2 3-1-.2-2.7 1.5-1.5 2 .2 1-3-2-1z",
  refresh: "M20 11a8 8 0 1 0-2.3 5.7M20 5v6h-6",
  play: "M8 5l11 7-11 7z",
  stop: "M6 6h12v12H6z",
  camera: "M4 8h3l2-3h6l2 3h3v11H4zM12 11a3 3 0 1 0 0 6 3 3 0 0 0 0-6z",
  trash: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
  close: "M6 6l12 12M18 6L6 18",
  chevron: "M9 6l6 6-6 6",
  back: "M15 6l-6 6 6 6",
  power: "M12 3v8M6.3 7a8 8 0 1 0 11.4 0",
  repeat: "M4 9h13l-3-3M20 15H7l3 3",
  key: "M14 10a4 4 0 1 1-1.2 2.8L4 21H2v-3h2v-2h2l2.8-2.8A4 4 0 0 1 14 10zM16 8h.01",
  link: "M10 14l4-4M8 12l-2 2a3 3 0 0 0 4 4l2-2M16 12l2-2a3 3 0 0 0-4-4l-2 2",
};
export type IconName = keyof typeof paths;
export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d={paths[name]} /></svg>;
}

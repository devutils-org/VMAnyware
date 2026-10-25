// The devutils mark (devutils-org.com/public/mark.svg), inlined so it needs no asset path.
export function BrandMark({ size = 24 }: { size?: number }) {
  return <svg className="brand-mark" aria-hidden="true" width={size} height={size} viewBox="0 0 62 62">
    <defs><linearGradient id="devutils-mark" x1="0" y1="0" x2="62" y2="62" gradientUnits="userSpaceOnUse"><stop stopColor="#e6fbff" /><stop offset=".55" stopColor="#c9d4ff" /><stop offset="1" stopColor="#c3a8ff" /></linearGradient></defs>
    <g fill="url(#devutils-mark)">
      <path d="M31 9.3C42.98 9.3 52.7 19.02 52.7 31c0 2.51-.43 4.93-1.22 7.18-3.45-.07-6.93 1.22-9.57 3.86-2.62 2.62-3.9 6.06-3.86 9.49A21.66 21.66 0 0 1 31 52.7C19.02 52.7 9.3 42.98 9.3 31S19.02 9.3 31 9.3Z" />
      <path fillRule="evenodd" d="M31 0c17.12 0 31 13.88 31 31 0 5.18-1.27 10.06-3.51 14.35 3.31 3.65 3.2 9.3-.32 12.82-3.52 3.52-9.17 3.63-12.82.32A30.86 30.86 0 0 1 31 62C13.88 62 0 48.12 0 31S13.88 0 31 0Zm0 4.65C16.45 4.65 4.65 16.45 4.65 31S16.45 57.35 31 57.35c4.24 0 8.24-1 11.78-2.77-1.1-3.25-.35-6.98 2.24-9.56 2.58-2.59 6.31-3.33 9.56-2.24A26.25 26.25 0 0 0 57.35 31C57.35 16.45 45.55 4.65 31 4.65Z" />
    </g>
  </svg>;
}

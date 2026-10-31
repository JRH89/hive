type IconName =
  "arrow" | "terminal" | "layers" | "keyboard" | "detach" | "check";

const paths: Record<IconName, React.ReactNode> = {
  arrow: (
    <>
      <path d="M5 12h14M13 6l6 6-6 6" />
    </>
  ),
  terminal: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="3" />
      <path d="m7 9 3 3-3 3m6 0h4" />
    </>
  ),
  layers: (
    <>
      <rect x="3" y="3" width="13" height="13" rx="2" />
      <path d="M8 20h11a2 2 0 0 0 2-2V8M7 7h5M7 11h3" />
    </>
  ),
  keyboard: (
    <>
      <rect x="2" y="5" width="20" height="14" rx="3" />
      <path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 12h.01M10 12h.01M14 12h.01M18 12h.01M7 15h10" />
    </>
  ),
  detach: (
    <>
      <path d="M13 4H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h8M10 12h11m-4-4 4 4-4 4" />
    </>
  ),
  check: <path d="m5 12 4 4L19 6" />,
};

export function Icon({ name, size = 20 }: { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths[name]}
    </svg>
  );
}

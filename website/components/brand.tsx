export function Brand({ compact = false }: { compact?: boolean }) {
  return (
    <span className="brand">
      <svg
        width="34"
        height="38"
        viewBox="0 0 34 38"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M17 2 32 10.5v17L17 36 2 27.5v-17L17 2Z"
          stroke="currentColor"
          strokeWidth="1.7"
        />
        <path
          d="m10 14 5 5-5 5M18 24h7"
          stroke="currentColor"
          strokeWidth="1.7"
          strokeLinecap="square"
          strokeLinejoin="miter"
        />
      </svg>
      {!compact && (
        <span>
          HIVE<span className="brand-period">.</span>
        </span>
      )}
    </span>
  );
}

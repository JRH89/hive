const leftCells = [
  [35, 100],
  [95, 100],
  [65, 47.5],
  [125, 47.5],
  [155, 100],
  [65, 152.5],
  [125, 152.5],
  [95, 205],
  [155, 205],
  [185, 152.5],
];
const cells = [...leftCells, ...leftCells.map(([x, y]) => [1160 - x, y + 75])];

/** Original decorative SVG. No image request, animation, or pointer interaction. */
export function HiveBackdrop() {
  return (
    <svg
      className="hero-hive-bg"
      viewBox="0 0 1160 430"
      fill="none"
      aria-hidden="true"
      focusable="false"
    >
      {cells.map(([x, y], index) => (
        <path
          key={`${x}-${y}`}
          d={`M${x} ${y - 35}l30 17.5v35l-30 17.5-30-17.5v-35Z`}
          className={
            index === 3 || index === 16 ? "hive-cell-accent" : "hive-cell"
          }
        />
      ))}
      <path className="hive-connection" d="M165 65V30h120M995 275v90H875" />
      <circle className="hive-node" cx="285" cy="30" r="2" />
      <circle className="hive-node" cx="875" cy="365" r="2" />
    </svg>
  );
}

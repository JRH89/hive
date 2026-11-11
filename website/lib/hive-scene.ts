/**
 * The hero's holographic hive: a curved honeycomb wall that breathes with an
 * energy wave, electric arcs that jump between cells, two spinning cores,
 * floating agent-status panels, a scrolling hex floor, rising light streaks
 * and drifting dust, all through a bloom pass.
 *
 * Loaded lazily by `components/hero-scene.tsx`, so three.js never blocks the
 * first paint. Everything created here is released by `dispose()`.
 */
import * as THREE from "three";
import { EffectComposer } from "three/addons/postprocessing/EffectComposer.js";
import { RenderPass } from "three/addons/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/addons/postprocessing/UnrealBloomPass.js";
import { OutputPass } from "three/addons/postprocessing/OutputPass.js";

export type HiveScene = { dispose(): void };

const BG = 0x111318;
const AMBER = new THREE.Color(0xf2b84b);
const HOT = new THREE.Color(0xffe2a0);
const GREEN = new THREE.Color(0x79c99e);

const FOV = 40;
const CAMERA_Z = 16;
const WALL_Z = -5;
const HEX_R = 0.95;

const VERT_UV =
  "varying vec2 vUv; void main() { vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }";

const additive = {
  transparent: true,
  depthWrite: false,
  blending: THREE.AdditiveBlending,
} as const;

/** Visible half-width of the view at depth `z`. */
function halfWidthAt(z: number, aspect: number) {
  return Math.tan(THREE.MathUtils.degToRad(FOV / 2)) * (CAMERA_Z - z) * aspect;
}

function glowTexture() {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 64;
  const ctx = canvas.getContext("2d")!;
  const g = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);
  g.addColorStop(0, "rgba(255,255,255,1)");
  g.addColorStop(0.25, "rgba(255,214,140,.75)");
  g.addColorStop(1, "rgba(0,0,0,0)");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, 64, 64);
  const tex = new THREE.CanvasTexture(canvas);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

/* ------------------------------------------------------------------------ */
/* Honeycomb wall                                                            */
/* ------------------------------------------------------------------------ */

type Cell = { x: number; y: number; z: number; col: number; row: number };

function wallZ(x: number) {
  // Concave towards the camera, so the edges of the hive wrap around you.
  return WALL_Z + 0.045 * x * x;
}

function buildCells(): Cell[] {
  const cells: Cell[] = [];
  const stepX = 1.5 * HEX_R;
  const stepY = Math.sqrt(3) * HEX_R;
  for (let col = -15; col <= 15; col++) {
    for (let row = -6; row <= 6; row++) {
      const x = col * stepX;
      const y = row * stepY + (col & 1 ? stepY / 2 : 0);
      cells.push({ x, y, z: wallZ(x), col, row });
    }
  }
  return cells;
}

const WALL_VERT = /* glsl */ `
attribute float aRim;
attribute float aSeed;
attribute float aBoost;
attribute vec3 aCenter;
uniform float uTime;
uniform float uClear;
varying float vRim;
varying float vSeed;
varying float vBoost;
varying float vFade;
varying vec3 vWorld;
void main() {
  vec3 p = position;
  // Each cell drifts along z on its own phase: the hive breathes.
  p.z += sin(uTime * 0.7 + aSeed * 6.2831) * 0.35 + aBoost * 0.6;
  vRim = aRim;
  vSeed = aSeed;
  vBoost = aBoost;
  vWorld = p;
  // Keep the column behind the headline quiet, and fade the top and bottom.
  float side = smoothstep(uClear * 0.45, uClear * 1.15, abs(aCenter.x));
  float vert = 1.0 - smoothstep(4.5, 9.5, abs(aCenter.y - 0.5));
  vFade = mix(0.04, 1.0, side) * vert;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.0);
}`;

const WALL_FRAG = /* glsl */ `
precision highp float;
uniform float uTime;
uniform float uBeam;
uniform vec3 uColor;
uniform vec3 uHot;
varying float vRim;
varying float vSeed;
varying float vBoost;
varying float vFade;
varying vec3 vWorld;
void main() {
  float w = fwidth(vRim) * 1.5;
  float edge = smoothstep(0.925 - w, 0.955, vRim) * (1.0 - smoothstep(0.985, 1.0 + w, vRim));
  float inner = smoothstep(0.58 - w, 0.6, vRim) * (1.0 - smoothstep(0.62, 0.64 + w, vRim));
  float d = length(vWorld.xy * vec2(0.8, 1.0));
  float wave = pow(0.5 + 0.5 * sin(d * 0.75 - uTime * 1.5), 14.0);
  float beam = exp(-abs(vWorld.x - uBeam) * 1.4);
  float flicker = 0.88 + 0.12 * sin(uTime * 9.0 + vSeed * 91.0);
  float scan = 0.75 + 0.25 * sin(vWorld.y * 36.0 - uTime * 5.0);
  float hotCell = step(0.94, vSeed) * (0.5 + 0.5 * sin(uTime * 1.3 + vSeed * 20.0));
  float lit = 0.05 + wave * 0.32 + beam * 0.28 + hotCell * 0.3 + vBoost * 1.5;
  vec3 col = uColor * (edge * (0.1 + lit) * 0.75 + inner * lit * 0.18);
  col += uColor * (1.0 - vRim) * 0.03 * lit * scan;
  col += uHot * vBoost * (edge * 0.9 + (1.0 - vRim) * 0.18);
  gl_FragColor = vec4(col * vFade * flicker, 1.0);
}`;

function buildWall(cells: Cell[]) {
  const perCell = 18; // 6 triangles: center, rim i, rim i+1
  const n = cells.length * perCell;
  const pos = new Float32Array(n * 3);
  const rim = new Float32Array(n);
  const seed = new Float32Array(n);
  const boost = new Float32Array(n);
  const center = new Float32Array(n * 3);
  const r = HEX_R * 0.9;
  let v = 0;
  for (const c of cells) {
    const s = Math.random();
    for (let i = 0; i < 6; i++) {
      const a0 = (i / 6) * Math.PI * 2;
      const a1 = ((i + 1) / 6) * Math.PI * 2;
      const pts: [number, number, number][] = [
        [c.x, c.y, 0],
        [c.x + Math.cos(a0) * r, c.y + Math.sin(a0) * r, 1],
        [c.x + Math.cos(a1) * r, c.y + Math.sin(a1) * r, 1],
      ];
      for (const [x, y, k] of pts) {
        pos.set([x, y, c.z], v * 3);
        center.set([c.x, c.y, c.z], v * 3);
        rim[v] = k;
        seed[v] = s;
        v++;
      }
    }
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  geo.setAttribute("aRim", new THREE.BufferAttribute(rim, 1));
  geo.setAttribute("aSeed", new THREE.BufferAttribute(seed, 1));
  geo.setAttribute("aCenter", new THREE.BufferAttribute(center, 3));
  const boostAttr = new THREE.BufferAttribute(boost, 1);
  boostAttr.setUsage(THREE.DynamicDrawUsage);
  geo.setAttribute("aBoost", boostAttr);
  const mat = new THREE.ShaderMaterial({
    vertexShader: WALL_VERT,
    fragmentShader: WALL_FRAG,
    ...additive,
    side: THREE.DoubleSide,
    uniforms: {
      uTime: { value: 0 },
      uBeam: { value: -100 },
      uClear: { value: 6 },
      uColor: { value: AMBER.clone() },
      uHot: { value: HOT.clone() },
    },
  });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.frustumCulled = false;
  return { mesh, geo, mat, boost, boostAttr, perCell };
}

/* ------------------------------------------------------------------------ */
/* Electric arcs                                                             */
/* ------------------------------------------------------------------------ */

const MAX_BOLTS = 14;
const BOLT_DEPTH = 5;
const BOLT_POINTS = (1 << BOLT_DEPTH) + 1;

type Bolt = {
  from: THREE.Vector3;
  to: THREE.Vector3;
  life: number;
  age: number;
  jitter: number;
  next: number;
  active: boolean;
};

const BOLT_VERT = /* glsl */ `
attribute float aSide;
attribute float aAlpha;
varying float vSide;
varying float vAlpha;
void main() {
  vSide = aSide;
  vAlpha = aAlpha;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}`;

const BOLT_FRAG = /* glsl */ `
precision highp float;
uniform vec3 uColor;
uniform vec3 uHot;
varying float vSide;
varying float vAlpha;
void main() {
  float x = abs(vSide);
  float core = exp(-x * x * 60.0);
  float halo = exp(-x * 3.5) * 0.45;
  vec3 col = uHot * core * 1.6 + uColor * halo * 0.6;
  gl_FragColor = vec4(col * vAlpha, 1.0);
}`;

function buildBolts() {
  const verts = MAX_BOLTS * BOLT_POINTS * 2;
  const pos = new Float32Array(verts * 3);
  const side = new Float32Array(verts);
  const alpha = new Float32Array(verts);
  const index: number[] = [];
  for (let b = 0; b < MAX_BOLTS; b++) {
    for (let i = 0; i < BOLT_POINTS; i++) {
      const v = (b * BOLT_POINTS + i) * 2;
      side[v] = -1;
      side[v + 1] = 1;
      if (i < BOLT_POINTS - 1) index.push(v, v + 1, v + 2, v + 1, v + 3, v + 2);
    }
  }
  const geo = new THREE.BufferGeometry();
  const posAttr = new THREE.BufferAttribute(pos, 3).setUsage(THREE.DynamicDrawUsage);
  const alphaAttr = new THREE.BufferAttribute(alpha, 1).setUsage(THREE.DynamicDrawUsage);
  geo.setAttribute("position", posAttr);
  geo.setAttribute("aSide", new THREE.BufferAttribute(side, 1));
  geo.setAttribute("aAlpha", alphaAttr);
  geo.setIndex(index);
  const mat = new THREE.ShaderMaterial({
    vertexShader: BOLT_VERT,
    fragmentShader: BOLT_FRAG,
    ...additive,
    side: THREE.DoubleSide,
    uniforms: { uColor: { value: AMBER.clone() }, uHot: { value: HOT.clone() } },
  });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.frustumCulled = false;
  const bolts: Bolt[] = Array.from({ length: MAX_BOLTS }, () => ({
    from: new THREE.Vector3(),
    to: new THREE.Vector3(),
    life: 0,
    age: 0,
    jitter: 0,
    next: 0,
    active: false,
  }));
  const pts = Array.from({ length: BOLT_POINTS }, () => new THREE.Vector3());

  /** Midpoint displacement, then a camera-facing ribbon around the path. */
  function shape(b: number, bolt: Bolt) {
    const len = bolt.from.distanceTo(bolt.to);
    pts[0].copy(bolt.from);
    pts[BOLT_POINTS - 1].copy(bolt.to);
    let step = BOLT_POINTS - 1;
    let amp = len * 0.22;
    while (step > 1) {
      const half = step >> 1;
      for (let i = half; i < BOLT_POINTS; i += step) {
        const a = pts[i - half];
        const c = pts[i + half];
        const dx = c.x - a.x;
        const dy = c.y - a.y;
        const l = Math.hypot(dx, dy) || 1;
        const off = (Math.random() - 0.5) * 2 * amp;
        pts[i].set(
          (a.x + c.x) / 2 + (-dy / l) * off,
          (a.y + c.y) / 2 + (dx / l) * off,
          (a.z + c.z) / 2 + (Math.random() - 0.5) * amp * 0.4,
        );
      }
      step = half;
      amp *= 0.55;
    }
    const width = 0.05 + len * 0.004;
    const fade = Math.sin(Math.min(bolt.age / bolt.life, 1) * Math.PI);
    const flick = 0.55 + 0.45 * Math.random();
    for (let i = 0; i < BOLT_POINTS; i++) {
      const a = pts[Math.max(i - 1, 0)];
      const c = pts[Math.min(i + 1, BOLT_POINTS - 1)];
      const dx = c.x - a.x;
      const dy = c.y - a.y;
      const l = Math.hypot(dx, dy) || 1;
      const taper = Math.sin((i / (BOLT_POINTS - 1)) * Math.PI) * 0.7 + 0.3;
      const nx = (-dy / l) * width * taper;
      const ny = (dx / l) * width * taper;
      const v = (b * BOLT_POINTS + i) * 2;
      const p = pts[i];
      pos.set([p.x - nx, p.y - ny, p.z, p.x + nx, p.y + ny, p.z], v * 3);
      alpha[v] = alpha[v + 1] = fade * flick;
    }
  }

  function clear(b: number) {
    alpha.fill(0, b * BOLT_POINTS * 2, (b + 1) * BOLT_POINTS * 2);
  }

  return { mesh, geo, mat, bolts, shape, clear, posAttr, alphaAttr };
}

/* ------------------------------------------------------------------------ */
/* Holographic agent panels                                                  */
/* ------------------------------------------------------------------------ */

type PanelState = {
  status: string;
  color: string;
  lines: [string, string][];
};

const PANELS: { agent: string; where: string; states: PanelState[] }[] = [
  {
    agent: "claude",
    where: "hive / api",
    states: [
      {
        status: "● WORKING",
        color: "#f2b84b",
        lines: [
          ["dim", "› simplify the session store"],
          ["code", "  edit  crates/daemon/src/session.rs"],
          ["add", "  +42"],
          ["del", "  −17"],
          ["dim", "  running cargo test…"],
        ],
      },
      {
        status: "✓ DONE",
        color: "#79c99e",
        lines: [
          ["dim", "› simplify the session store"],
          ["ok", "  ✓ 214 tests passed"],
          ["code", "  3 files changed"],
          ["dim", "  ready for review"],
        ],
      },
    ],
  },
  {
    agent: "codex",
    where: "worktree · feat/notes",
    states: [
      {
        status: "◐ NEEDS YOU",
        color: "#ff9d5c",
        lines: [
          ["dim", "› wire up the notes pane"],
          ["code", "  wants to run:"],
          ["warn", "  $ cargo test -p hive-notes"],
          ["dim", "  allow?  [y] yes   [n] no"],
        ],
      },
      {
        status: "● WORKING",
        color: "#f2b84b",
        lines: [
          ["dim", "› wire up the notes pane"],
          ["code", "  edit  crates/notes/src/vault.rs"],
          ["add", "  +88"],
          ["dim", "  thinking…"],
        ],
      },
    ],
  },
  {
    agent: "gemini",
    where: "hive / docs",
    states: [
      {
        status: "● WORKING",
        color: "#f2b84b",
        lines: [
          ["dim", "› update the keys reference"],
          ["code", "  read  content/docs/keys.tsx"],
          ["dim", "  drafting…"],
        ],
      },
      {
        status: "✓ DONE",
        color: "#79c99e",
        lines: [
          ["dim", "› update the keys reference"],
          ["ok", "  ✓ 2 files changed"],
          ["dim", "  waiting for you"],
        ],
      },
    ],
  },
  {
    agent: "hived",
    where: "daemon",
    states: [
      {
        status: "● 4 SESSIONS",
        color: "#f2b84b",
        lines: [
          ["code", "  claude    hive/api        working"],
          ["code", "  codex     feat/notes      needs you"],
          ["code", "  gemini    hive/docs       working"],
          ["dim", "  detached · still running"],
        ],
      },
    ],
  },
];

const INK: Record<string, string> = {
  code: "#e8eaf0",
  dim: "#8992a3",
  add: "#79c99e",
  del: "#ff6b6b",
  ok: "#79c99e",
  warn: "#ffd27a",
};

const PANEL_W = 3.6;
const PANEL_H = 2.25;
const TEX_W = 1024;
const TEX_H = 640;

function drawPanel(
  ctx: CanvasRenderingContext2D,
  agent: string,
  where: string,
  state: PanelState,
  font: string,
) {
  ctx.clearRect(0, 0, TEX_W, TEX_H);
  ctx.textBaseline = "middle";
  ctx.shadowColor = "rgba(242,184,75,.8)";
  ctx.shadowBlur = 10;
  ctx.font = `700 34px ${font}`;
  ctx.fillStyle = "#f2b84b";
  ctx.textAlign = "left";
  ctx.fillText(agent, 48, 62);
  const agentW = ctx.measureText(agent).width;
  ctx.font = `500 24px ${font}`;
  ctx.fillStyle = "#8992a3";
  ctx.fillText(`  ${where}`, 48 + agentW, 64);
  ctx.textAlign = "right";
  ctx.font = `700 24px ${font}`;
  ctx.fillStyle = state.color;
  ctx.shadowColor = state.color;
  ctx.fillText(state.status, TEX_W - 48, 62);
  ctx.textAlign = "left";
  ctx.shadowBlur = 0;
  ctx.fillStyle = "rgba(242,184,75,.45)";
  ctx.fillRect(48, 104, TEX_W - 96, 2);
  ctx.font = `500 30px ${font}`;
  state.lines.forEach(([kind, text], i) => {
    const y = 170 + i * 66;
    if (kind === "add" || kind === "del" || kind === "warn") {
      ctx.fillStyle =
        kind === "add"
          ? "rgba(121,201,158,.12)"
          : kind === "del"
            ? "rgba(255,107,107,.12)"
            : "rgba(255,210,122,.1)";
      ctx.fillRect(44, y - 28, TEX_W - 88, 56);
      ctx.fillStyle = INK[kind];
      ctx.fillRect(44, y - 28, 5, 56);
    }
    ctx.shadowBlur = 8;
    ctx.shadowColor = kind === "ok" || kind === "add" ? "rgba(121,201,158,.7)" : "rgba(242,184,75,.6)";
    ctx.fillStyle = INK[kind] ?? INK.code;
    ctx.fillText(text, 60, y);
  });
}

const PANEL_FRAG = /* glsl */ `
precision highp float;
varying vec2 vUv;
uniform sampler2D map;
uniform float uTime, uScan, uSeed, uGlow;
uniform vec3 uTint;
void main() {
  vec4 tex = texture2D(map, vUv);
  vec2 e = min(vUv, 1.0 - vUv);
  float aspect = ${(PANEL_W / PANEL_H).toFixed(3)};
  float m = min(e.x, e.y / aspect);
  float edge = smoothstep(0.01, 0.0, m);
  float corner = step(min(e.x, e.y / aspect), 0.022) * step(max(e.x, e.y / aspect), 0.08);
  float scan = 0.82 + 0.18 * sin(vUv.y * 420.0 - uTime * 6.0);
  float band = uScan > 0.0 && uScan < 1.0 ? smoothstep(0.06, 0.0, abs(1.0 - vUv.y - uScan)) : 0.0;
  float sweep = smoothstep(0.035, 0.0, abs(vUv.y - fract(-uTime * 0.15 + uSeed))) * 0.18;
  float glitch = step(0.985, fract(sin(floor(uTime * 12.0) + uSeed * 7.0) * 43758.5));
  vec3 col = tex.rgb * tex.a * scan * (1.0 - glitch * 0.6);
  col += uTint * (edge * 0.8 + corner * 1.6 + sweep + 0.035);
  col *= 0.28 + 0.32 * uGlow;
  col += uTint * band * 0.7 + vec3(1.0) * band * tex.a * 0.25;
  gl_FragColor = vec4(col, 1.0);
}`;

/* ------------------------------------------------------------------------ */
/* Floor                                                                     */
/* ------------------------------------------------------------------------ */

const FLOOR_FRAG = /* glsl */ `
precision highp float;
varying vec2 vUv;
uniform float uTime;
uniform vec3 uColor;
float hexDist(vec2 p) {
  p = abs(p);
  return max(dot(p, normalize(vec2(1.0, 1.7320508))), p.x);
}
void main() {
  vec2 uv = vUv * vec2(90.0, 45.0);
  uv.y += uTime * 0.9;
  vec2 r = vec2(1.0, 1.7320508);
  vec2 h = r * 0.5;
  vec2 a = mod(uv, r) - h;
  vec2 b = mod(uv - h, r) - h;
  vec2 g = dot(a, a) < dot(b, b) ? a : b;
  float d = 0.5 - hexDist(g);
  float w = fwidth(d) * 1.2;
  float line = 1.0 - smoothstep(0.0, w + 0.015, d);
  float lod = 1.0 - smoothstep(0.05, 0.25, fwidth(uv.y));
  float near = smoothstep(0.0, 0.18, vUv.y);
  float far = 1.0 - smoothstep(0.35, 0.75, vUv.y);
  float side = 1.0 - smoothstep(0.25, 0.5, abs(vUv.x - 0.5));
  float pulse = pow(0.5 + 0.5 * sin(vUv.y * 60.0 - uTime * 2.5), 6.0);
  gl_FragColor = vec4(uColor * line * lod * near * far * side * (0.18 + pulse * 0.45), 1.0);
}`;

/* ------------------------------------------------------------------------ */
/* Scene                                                                     */
/* ------------------------------------------------------------------------ */

export function createHiveScene(
  container: HTMLElement,
  options: { reduced: boolean },
): HiveScene {
  const owned: { dispose(): void }[] = [];
  const own = <T extends { dispose(): void }>(x: T) => (owned.push(x), x);

  const renderer = new THREE.WebGLRenderer({
    antialias: false,
    alpha: false,
    powerPreference: "high-performance",
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
  renderer.setClearColor(BG, 1);
  renderer.domElement.style.cssText = "display:block;width:100%;height:100%";
  renderer.domElement.setAttribute("aria-hidden", "true");
  container.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  // The clear colour comes out double-encoded through the composer; a scene
  // background is colour-managed and matches the page exactly.
  scene.background = new THREE.Color(BG);
  scene.fog = new THREE.FogExp2(BG, 0.028);
  const camera = new THREE.PerspectiveCamera(FOV, 1, 0.1, 200);
  camera.position.set(0, 0.3, CAMERA_Z);

  const glow = own(glowTexture());
  const font =
    (getComputedStyle(document.documentElement).getPropertyValue("--font-geist-mono").trim() ||
      "") + ", ui-monospace, monospace";

  // Honeycomb wall.
  const cells = buildCells();
  const wall = buildWall(cells);
  own(wall.geo);
  own(wall.mat);
  scene.add(wall.mesh);
  const cellBoost = new Float32Array(cells.length);

  // Electric arcs.
  const arcs = buildBolts();
  own(arcs.geo);
  own(arcs.mat);
  scene.add(arcs.mesh);

  // Cores on each flank: wireframe icosahedron, inner octahedron, hex rings.
  const cores = [-1, 1].map((side) => {
    const group = new THREE.Group();
    const shell = new THREE.LineSegments(
      own(new THREE.WireframeGeometry(own(new THREE.IcosahedronGeometry(1.15, 1)))),
      own(new THREE.LineBasicMaterial({ color: AMBER, ...additive, opacity: 0.85 })),
    );
    const heart = new THREE.LineSegments(
      own(new THREE.EdgesGeometry(own(new THREE.OctahedronGeometry(0.55)))),
      own(new THREE.LineBasicMaterial({ color: HOT, ...additive, opacity: 0.95 })),
    );
    const halo = new THREE.Sprite(
      own(new THREE.SpriteMaterial({ map: glow, color: 0xffc860, ...additive, opacity: 0.7 })),
    );
    halo.scale.setScalar(2.6);
    const ringMat = own(
      new THREE.MeshBasicMaterial({ color: AMBER, ...additive, opacity: 0.55, side: THREE.DoubleSide }),
    );
    const rings = [1.7, 2.05, 2.5].map((r, i) => {
      const ring = new THREE.Mesh(own(new THREE.RingGeometry(r, r + 0.025, 6)), ringMat);
      ring.rotation.set(Math.PI / 2 + (i - 1) * 0.55, i * 0.8, 0);
      group.add(ring);
      return ring;
    });
    const orbitPos = new Float32Array(48 * 3);
    const orbitGeo = own(new THREE.BufferGeometry());
    orbitGeo.setAttribute("position", new THREE.BufferAttribute(orbitPos, 3));
    const orbit = new THREE.Points(
      orbitGeo,
      own(new THREE.PointsMaterial({ map: glow, size: 0.16, color: 0xffd27a, ...additive, opacity: 0.9 })),
    );
    group.add(shell, heart, halo, orbit);
    scene.add(group);
    return { side, group, shell, heart, halo, rings, orbitPos, orbitGeo };
  });

  // Holographic panels.
  const panelGeo = own(new THREE.PlaneGeometry(PANEL_W, PANEL_H));
  const panelLayout = [
    // [side, x fraction of half-width, y, z, yaw, scale]
    [-1, 0.62, 2.35, -1.5, 0.38, 0.9],
    [1, 0.6, 2.2, -1.2, -0.38, 0.92],
    [-1, 0.86, -1.9, -3.5, 0.45, 0.95],
    [1, 0.84, -2.1, -3.8, -0.45, 0.95],
  ] as const;
  const panels = PANELS.map((def, i) => {
    const canvas = document.createElement("canvas");
    canvas.width = TEX_W;
    canvas.height = TEX_H;
    const ctx = canvas.getContext("2d")!;
    const tex = own(new THREE.CanvasTexture(canvas));
    tex.colorSpace = THREE.SRGBColorSpace;
    tex.anisotropy = renderer.capabilities.getMaxAnisotropy();
    const mat = own(
      new THREE.ShaderMaterial({
        vertexShader: VERT_UV,
        fragmentShader: PANEL_FRAG,
        ...additive,
        side: THREE.DoubleSide,
        uniforms: {
          map: { value: tex },
          uTime: { value: 0 },
          uScan: { value: 0 },
          uSeed: { value: i * 0.37 },
          uGlow: { value: 0.4 },
          uTint: { value: AMBER.clone() },
        },
      }),
    );
    const mesh = new THREE.Mesh(panelGeo, mat);
    const group = new THREE.Group();
    group.add(mesh);
    scene.add(group);
    const [side, fx, y, z, yaw, scale] = panelLayout[i];
    group.scale.setScalar(scale);
    let state = 0;
    const draw = () => {
      const s = def.states[state % def.states.length];
      drawPanel(ctx, def.agent, def.where, s, font);
      mat.uniforms.uTint.value.copy(s.color === "#79c99e" ? GREEN : AMBER);
      tex.needsUpdate = true;
    };
    draw();
    return {
      group,
      mat,
      side,
      fx,
      y,
      z,
      yaw,
      phase: i * 2.3 + 1,
      period: 7 + i * 1.3,
      advance() {
        state++;
        draw();
      },
      redraw: draw,
      flipped: -1,
    };
  });
  document.fonts?.ready.then(() => panels.forEach((p) => p.redraw()));

  // Hex floor.
  const floorMat = own(
    new THREE.ShaderMaterial({
      vertexShader: VERT_UV,
      fragmentShader: FLOOR_FRAG,
      ...additive,
      uniforms: { uTime: { value: 0 }, uColor: { value: AMBER.clone().multiplyScalar(0.8) } },
    }),
  );
  const floor = new THREE.Mesh(own(new THREE.PlaneGeometry(220, 110)), floorMat);
  floor.rotation.x = -Math.PI / 2;
  floor.position.set(0, -5.2, -45);
  scene.add(floor);

  // Rising light streaks along the flanks.
  const streakMat = own(
    new THREE.ShaderMaterial({
      vertexShader:
        "varying vec2 vUv; void main() { vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * instanceMatrix * vec4(position, 1.0); }",
      fragmentShader: /* glsl */ `
        precision highp float; varying vec2 vUv; uniform vec3 uColor; uniform vec3 uHot;
        void main() {
          float y = clamp(vUv.y, 0.0, 1.0);
          float a = pow(y, 2.5) * smoothstep(0.0, 0.5, 1.0 - abs(vUv.x - 0.5) * 2.0);
          gl_FragColor = vec4(mix(uColor * 0.6, uHot, pow(y, 6.0)) * a, 1.0);
        }`,
      ...additive,
      uniforms: { uColor: { value: AMBER.clone() }, uHot: { value: HOT.clone() } },
    }),
  );
  const STREAKS = 60;
  const streaks = new THREE.InstancedMesh(own(new THREE.PlaneGeometry(0.035, 1.6)), streakMat, STREAKS);
  streaks.frustumCulled = false;
  scene.add(streaks);
  const streakData = Array.from({ length: STREAKS }, () => ({
    side: Math.random() < 0.5 ? -1 : 1,
    fx: 0.35 + 0.75 * Math.random(),
    z: -1 - 14 * Math.random(),
    y: -5 + 11 * Math.random(),
    v: 1.2 + 2.6 * Math.random(),
    len: 0.5 + 1.3 * Math.random(),
  }));
  const tmpM = new THREE.Matrix4();
  const tmpQ = new THREE.Quaternion();
  const tmpP = new THREE.Vector3();
  const tmpS = new THREE.Vector3();

  // Drifting dust.
  const DUST = 800;
  const dustPos = new Float32Array(DUST * 3);
  for (let i = 0; i < DUST; i++) {
    dustPos.set([(Math.random() - 0.5) * 70, (Math.random() - 0.4) * 26, -4 - 60 * Math.random()], i * 3);
  }
  const dustGeo = own(new THREE.BufferGeometry());
  dustGeo.setAttribute("position", new THREE.BufferAttribute(dustPos, 3));
  scene.add(
    new THREE.Points(
      dustGeo,
      own(new THREE.PointsMaterial({ map: glow, size: 0.12, color: 0xe8c890, ...additive, opacity: 0.5 })),
    ),
  );

  // Post-processing: bloom is what turns thin lines into electricity.
  const composer = own(
    new EffectComposer(renderer, new THREE.WebGLRenderTarget(1, 1, { type: THREE.HalfFloatType, samples: 4 })),
  );
  composer.addPass(new RenderPass(scene, camera));
  const bloom = own(new UnrealBloomPass(new THREE.Vector2(1, 1), 0.7, 0.25, 0.3));
  composer.addPass(bloom);
  composer.addPass(new OutputPass());

  // Layout, recomputed on resize.
  let aspect = 1;
  let narrow = false;
  const corePos = [new THREE.Vector3(), new THREE.Vector3()];
  function layout() {
    const w = container.clientWidth;
    const h = container.clientHeight;
    if (!w || !h) return;
    renderer.setSize(w, h, false);
    composer.setSize(w, h);
    bloom.resolution.set(w, h);
    aspect = w / h;
    narrow = aspect < 1.05;
    camera.aspect = aspect;
    camera.updateProjectionMatrix();
    // The headline spans roughly 640px; keep that column clear at the wall.
    const clearFrac = Math.min(0.95, 340 / w);
    wall.mat.uniforms.uClear.value = halfWidthAt(WALL_Z, aspect) * clearFrac * 2;
    cores.forEach((c, i) => {
      const z = -1.5;
      const x = narrow ? c.side * halfWidthAt(z, aspect) * 0.95 : c.side * halfWidthAt(z, aspect) * 0.74;
      corePos[i].set(x, narrow ? 3.6 : 0.4, z);
      c.group.position.copy(corePos[i]);
      c.group.scale.setScalar(narrow ? 0.7 : 1);
    });
    panels.forEach((p) => {
      p.group.visible = !narrow;
      p.group.position.set(p.side * halfWidthAt(p.z, aspect) * p.fx, p.y, p.z);
    });
    if (options.reduced) render(0);
  }
  const resizer = new ResizeObserver(layout);
  resizer.observe(container);

  // Pointer parallax and scroll dolly.
  const pointer = new THREE.Vector2();
  const pointerTarget = new THREE.Vector2();
  const onPointer = (e: PointerEvent) => {
    pointerTarget.set((e.clientX / window.innerWidth) * 2 - 1, (e.clientY / window.innerHeight) * 2 - 1);
  };
  if (!options.reduced) window.addEventListener("pointermove", onPointer, { passive: true });

  // Arc scheduling.
  let spawnIn = 0.2;
  const nearestCells = (p: THREE.Vector3, minD: number, maxD: number) =>
    cells
      .map((c, i) => ({ i, d: Math.hypot(c.x - p.x, c.y - p.y) }))
      .filter((c) => c.d > minD && c.d < maxD)
      .map((c) => c.i);
  const cellPoint = (i: number, out: THREE.Vector3) => out.set(cells[i].x, cells[i].y, cells[i].z + 0.05);
  const visibleCell = (i: number) => {
    const c = cells[i];
    const clear = wall.mat.uniforms.uClear.value as number;
    return Math.abs(c.x) > clear * 0.75 && Math.abs(c.x) < halfWidthAt(c.z, aspect) * 1.05 && Math.abs(c.y - 0.5) < 5.2;
  };
  function spawnBolt() {
    const slot = arcs.bolts.findIndex((b) => !b.active);
    if (slot < 0) return;
    const bolt = arcs.bolts[slot];
    const fromCore = Math.random() < 0.45;
    let endCell = -1;
    if (fromCore) {
      const core = Math.random() < 0.5 ? 0 : 1;
      bolt.from.copy(corePos[core]);
      const choices = nearestCells(corePos[core], 1.8, 5.5).filter(visibleCell);
      if (!choices.length) return;
      endCell = choices[(Math.random() * choices.length) | 0];
      cellPoint(endCell, bolt.to);
    } else {
      const pool = cells.map((_, i) => i).filter(visibleCell);
      if (!pool.length) return;
      const start = pool[(Math.random() * pool.length) | 0];
      cellPoint(start, bolt.from);
      const choices = nearestCells(bolt.from, 1.5, 4.5).filter(visibleCell);
      if (!choices.length) return;
      endCell = choices[(Math.random() * choices.length) | 0];
      cellPoint(endCell, bolt.to);
      cellBoost[start] = Math.max(cellBoost[start], 0.9);
    }
    cellBoost[endCell] = 1;
    bolt.active = true;
    bolt.age = 0;
    bolt.life = 0.18 + Math.random() * 0.35;
    bolt.next = 0;
  }

  // Animation.
  let time = 3.2;
  let last = 0;
  let running = false;
  let visible = true;
  let frame = 0;

  function render(dt: number) {
    time += dt;
    const t = time;

    pointer.lerp(pointerTarget, 1 - Math.exp(-dt * 3));
    const scroll = Math.min(window.scrollY, 900);
    camera.position.set(
      Math.sin(t * 0.11) * 0.5 + pointer.x * 0.9,
      0.3 + Math.sin(t * 0.17) * 0.2 - pointer.y * 0.45 + scroll * 0.0015,
      CAMERA_Z + scroll * 0.004,
    );
    camera.lookAt(0, 0.2 - scroll * 0.001, -3);

    // Wall: energy wave, beam and decaying cell boosts.
    wall.mat.uniforms.uTime.value = t;
    const beamCycle = (t % 9) / 3.2;
    const hw = halfWidthAt(WALL_Z, aspect);
    wall.mat.uniforms.uBeam.value = beamCycle < 1 ? -hw * 1.2 + beamCycle * hw * 2.4 : -100;
    const decay = Math.exp(-dt * 3.2);
    for (let i = 0; i < cells.length; i++) {
      const b = (cellBoost[i] *= decay);
      wall.boost.fill(b, i * wall.perCell, (i + 1) * wall.perCell);
    }
    wall.boostAttr.needsUpdate = true;

    // Arcs.
    if (!options.reduced) {
      spawnIn -= dt;
      if (spawnIn <= 0) {
        spawnBolt();
        if (Math.random() < 0.3) spawnBolt();
        spawnIn = 0.08 + Math.random() * 0.35;
      }
    } else if (dt === 0 && !arcs.bolts.some((b) => b.active)) {
      for (let i = 0; i < 6; i++) spawnBolt();
      arcs.bolts.forEach((b) => (b.age = b.life * 0.5));
    }
    arcs.bolts.forEach((bolt, i) => {
      if (!bolt.active) return;
      bolt.age += dt;
      if (bolt.age >= bolt.life) {
        bolt.active = false;
        arcs.clear(i);
        return;
      }
      bolt.next -= dt;
      if (bolt.next <= 0 || dt === 0) {
        arcs.shape(i, bolt);
        bolt.next = 0.035 + Math.random() * 0.04;
      }
    });
    arcs.posAttr.needsUpdate = true;
    arcs.alphaAttr.needsUpdate = true;

    // Cores.
    cores.forEach((c, k) => {
      const s = c.side;
      c.shell.rotation.set(t * 0.13 * s, t * 0.21, 0);
      c.heart.rotation.set(-t * 0.4, t * 0.55 * s, t * 0.2);
      const beat = Math.pow(Math.max(0, Math.sin(t * 2.2 + k * 1.7)), 8);
      (c.halo.material as THREE.SpriteMaterial).opacity = 0.5 + 0.5 * beat;
      c.halo.scale.setScalar(2.3 + 0.9 * beat);
      c.rings[0].rotation.z = t * 0.5 * s;
      c.rings[1].rotation.z = -t * 0.35 * s;
      c.rings[2].rotation.z = t * 0.9 * s;
      c.group.position.y = corePos[k].y + Math.sin(t * 0.6 + k) * 0.2;
      for (let i = 0; i < 48; i++) {
        const a = t * (0.3 + (i % 7) * 0.06) + 2.399 * i;
        const r = 1.5 + (i % 5) * 0.22;
        const tilt = ((i % 3) - 1) * 0.6;
        c.orbitPos.set([Math.cos(a) * r, Math.sin(a) * r * Math.sin(tilt), Math.sin(a) * r * Math.cos(tilt)], i * 3);
      }
      c.orbitGeo.attributes.position.needsUpdate = true;
    });

    // Panels: bob, and every few seconds a scan band rewrites the status.
    panels.forEach((p, i) => {
      const cycle = (t + p.phase) % p.period;
      const scan = cycle < 1.4 ? cycle / 1.4 : 0;
      const cycleIndex = Math.floor((t + p.phase) / p.period);
      if (p.flipped !== cycleIndex) {
        if (p.flipped !== -1) p.advance();
        p.flipped = cycleIndex;
      }
      p.mat.uniforms.uTime.value = t;
      p.mat.uniforms.uScan.value = scan;
      p.mat.uniforms.uGlow.value = scan > 0 ? 1 : 0.55;
      const r = t * 0.4 + i;
      p.group.position.y = p.y + Math.sin(r) * 0.15;
      p.group.rotation.set(Math.sin(r * 0.6) * 0.04, p.yaw + Math.sin(r * 0.8) * 0.06, 0);
    });

    // Floor, streaks and dust.
    floorMat.uniforms.uTime.value = t;
    streakData.forEach((s, i) => {
      s.y += s.v * dt;
      if (s.y > 7) {
        s.y = -5.5 - Math.random() * 2;
        s.fx = 0.35 + 0.75 * Math.random();
      }
      tmpP.set(s.side * halfWidthAt(s.z, aspect) * s.fx, s.y, s.z);
      tmpS.set(1, s.len, 1);
      streaks.setMatrixAt(i, tmpM.compose(tmpP, tmpQ, tmpS));
    });
    streaks.instanceMatrix.needsUpdate = true;
    for (let i = 0; i < DUST; i++) {
      let z = dustPos[i * 3 + 2] + 1.8 * dt;
      if (z > -4) z -= 60;
      dustPos[i * 3 + 2] = z;
    }
    dustGeo.attributes.position.needsUpdate = true;

    composer.render();
  }

  function tick() {
    if (!visible || document.hidden) {
      running = false;
      return;
    }
    frame = requestAnimationFrame(tick);
    const now = performance.now();
    const dt = Math.min((now - last) / 1000, 0.1);
    last = now;
    render(dt);
  }
  function start() {
    if (running || options.reduced) return;
    running = true;
    last = performance.now();
    frame = requestAnimationFrame(tick);
  }
  const io = new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    if (visible) start();
  });
  io.observe(container);
  const onVisibility = () => {
    if (!document.hidden) start();
  };
  document.addEventListener("visibilitychange", onVisibility);

  layout();
  if (options.reduced) render(0);
  else start();

  return {
    dispose() {
      cancelAnimationFrame(frame);
      running = false;
      resizer.disconnect();
      io.disconnect();
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("pointermove", onPointer);
      owned.forEach((o) => o.dispose());
      renderer.dispose();
      renderer.domElement.remove();
    },
  };
}

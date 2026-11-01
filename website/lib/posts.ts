// Blog posts, newest first. The body of each is content/blog/<slug>.tsx.

export type Post = {
  slug: string;
  title: string;
  /** Meta description and the summary on the blog index (≤160 chars). */
  description: string;
  /** ISO 8601. */
  date: string;
  updated?: string;
  tags: string[];
  minutes: number;
};

export const posts: Post[] = [
  {
    slug: "get-notified-when-claude-code-needs-input",
    title: "How to get notified when Claude Code or Codex needs input",
    description:
      "Get a desktop notification when Claude Code or Codex stops at a permission prompt or finishes: Claude Code hooks, Codex settings, and one setup for all agents.",
    date: "2026-11-01T12:40:00-07:00",
    tags: ["Guide", "Claude Code", "Codex"],
    minutes: 5,
  },
  {
    slug: "keep-claude-code-running-after-closing-terminal",
    title: "How to keep Claude Code running after you close the terminal",
    description:
      "Why closing the terminal or SSH kills Claude Code and Codex, and four fixes: tmux, headless mode, Hive, and an always-on machine. Plus how to resume.",
    date: "2026-11-01T12:30:00-07:00",
    tags: ["Guide", "Claude Code"],
    minutes: 5,
  },
  {
    slug: "review-ai-agent-changes-before-commit",
    title: "How to review an AI coding agent's changes before you commit",
    description:
      "A five-step review routine for code written by Claude Code, Codex and other agents: isolate, overview, read by hunk, what to watch for, and run it yourself.",
    date: "2026-11-01T12:20:00-07:00",
    tags: ["Guide", "Git", "Workflow"],
    minutes: 6,
  },
  {
    slug: "zellij-vs-hive-for-ai-coding-agents",
    title: "Zellij or Hive for AI coding agents?",
    description:
      "Zellij and Hive compared for running Claude Code, Codex and other agents: what Zellij does well, what agents need, and which to use when.",
    date: "2026-11-01T12:10:00-07:00",
    tags: ["Comparison"],
    minutes: 5,
  },
  {
    slug: "hive-0-4-spaces-layouts-integrations",
    title: "Hive 0.4: spaces, layouts and integrations",
    description:
      "What changed in Hive 0.4: macOS and Windows, spaces that follow your cd, worktree terminals, right-click menus, layouts, and optional integrations.",
    date: "2026-10-31T22:45:00-07:00",
    tags: ["Release"],
    minutes: 4,
  },
  {
    slug: "run-multiple-ai-coding-agents",
    title: "How to run several AI coding agents at once",
    description:
      "Four rules for running Claude Code, Codex and other agents in parallel: separate checkouts, visible status, sessions that survive, and one place per project.",
    date: "2026-10-31T22:40:00-07:00",
    tags: ["Guide", "Workflow"],
    minutes: 6,
  },
  {
    slug: "git-worktrees-for-ai-coding-agents",
    title: "Git worktrees for AI coding agents: a practical guide",
    description:
      "Give every coding agent its own checkout with git worktree: the commands, the pitfalls (dependencies, .env files, branches) and how to clean up safely.",
    date: "2026-10-31T22:35:00-07:00",
    tags: ["Guide", "Git"],
    minutes: 7,
  },
  {
    slug: "tmux-vs-hive-for-ai-coding-agents",
    title: "tmux or Hive for Claude Code and Codex?",
    description:
      "An honest comparison of tmux and Hive for running AI coding agents: what tmux does well, what Hive adds, and when to use which (or both).",
    date: "2026-10-31T22:30:00-07:00",
    tags: ["Comparison"],
    minutes: 5,
  },
];

export const postBySlug = (slug: string) => posts.find((p) => p.slug === slug);

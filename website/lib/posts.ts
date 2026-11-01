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

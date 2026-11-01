// Maps a post slug to its body component (content/blog/<slug>.tsx).

import type { ComponentType } from "react";
import Hive04 from "@/content/blog/hive-0-4-spaces-layouts-integrations";
import RunMultiple from "@/content/blog/run-multiple-ai-coding-agents";
import Worktrees from "@/content/blog/git-worktrees-for-ai-coding-agents";
import TmuxVsHive from "@/content/blog/tmux-vs-hive-for-ai-coding-agents";

export const bodies: Record<string, ComponentType> = {
  "hive-0-4-spaces-layouts-integrations": Hive04,
  "run-multiple-ai-coding-agents": RunMultiple,
  "git-worktrees-for-ai-coding-agents": Worktrees,
  "tmux-vs-hive-for-ai-coding-agents": TmuxVsHive,
};

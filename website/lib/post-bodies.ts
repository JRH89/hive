// Maps a post slug to its body component (content/blog/<slug>.tsx).

import type { ComponentType } from "react";
import Hive04 from "@/content/blog/hive-0-4-spaces-layouts-integrations";
import RunMultiple from "@/content/blog/run-multiple-ai-coding-agents";
import Worktrees from "@/content/blog/git-worktrees-for-ai-coding-agents";
import TmuxVsHive from "@/content/blog/tmux-vs-hive-for-ai-coding-agents";
import Notify from "@/content/blog/get-notified-when-claude-code-needs-input";
import KeepRunning from "@/content/blog/keep-claude-code-running-after-closing-terminal";
import Review from "@/content/blog/review-ai-agent-changes-before-commit";
import Zellij from "@/content/blog/zellij-vs-hive-for-ai-coding-agents";

export const bodies: Record<string, ComponentType> = {
  "hive-0-4-spaces-layouts-integrations": Hive04,
  "run-multiple-ai-coding-agents": RunMultiple,
  "git-worktrees-for-ai-coding-agents": Worktrees,
  "tmux-vs-hive-for-ai-coding-agents": TmuxVsHive,
  "get-notified-when-claude-code-needs-input": Notify,
  "keep-claude-code-running-after-closing-terminal": KeepRunning,
  "review-ai-agent-changes-before-commit": Review,
  "zellij-vs-hive-for-ai-coding-agents": Zellij,
};

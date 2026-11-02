// Maps a docs slug to its body component (content/docs/<slug>.tsx).

import type { ComponentType } from "react";
import GettingStarted from "@/content/docs/getting-started";
import Interface from "@/content/docs/interface";
import Keys from "@/content/docs/keys";
import Configuration from "@/content/docs/configuration";
import Cli from "@/content/docs/cli";
import AgentStatus from "@/content/docs/agent-status";
import Worktrees from "@/content/docs/worktrees";
import Daemon from "@/content/docs/daemon";
import Troubleshooting from "@/content/docs/troubleshooting";

export const docBodies: Record<string, ComponentType> = {
  "getting-started": GettingStarted,
  interface: Interface,
  keys: Keys,
  configuration: Configuration,
  cli: Cli,
  "agent-status": AgentStatus,
  worktrees: Worktrees,
  daemon: Daemon,
  troubleshooting: Troubleshooting,
};

# Troubleshooting

Start with:

```bash
hive doctor
```

It shows the config file and whether it's valid, the socket path, whether the daemon is running, the log location, what colour and glyph support was detected, and which agents are on your `PATH`. It changes nothing and never starts the daemon. It exits non-zero if it found a problem.

Logs are in `~/.local/state/hive/`: `hived.log` for the daemon and `hive.log` for the client. For more detail, restart with debug logging:

```bash
hive daemon stop && HIVE_LOG=debug hive
```

Logs never contain environment variables or agent output.

## Common problems

| Symptom | Cause and fix |
|---|---|
| `daemon speaks protocol vN, this client speaks vM` | A daemon from an older version is still running. `hive daemon stop` (ends its sessions), then run `hive` again |
| `` `claude` (agent `claude`) was not found on PATH `` | The agent isn't installed or isn't on the `PATH` of the shell you ran `hive` from. Check `hive doctor`; set an absolute `command` under `[agents.<name>]` if needed |
| `hive: invalid config: …` | The message names the key. Fix it, or check with `hive doctor`. A config error never leaves a broken screen behind |
| `Ctrl+Space` does nothing | Your desktop or terminal uses it (input-method switching often does). Pick another leader: `[keys] leader = "ctrl+a"` |
| Keys meant for the agent are swallowed | Only `Ctrl+Space` is intercepted. To send it to the agent, press it twice |
| Boxes, question marks or misaligned borders | The font or locale lacks the Unicode glyphs. Set `[theme] glyphs = "ascii"`, or use a UTF-8 locale |
| Colours look wrong or garish | Force a depth (`[theme] colors = "256"`), try `theme = "ansi"` to use your terminal's palette, or `theme = "hive-light"` on a light background |
| Sessions show `◌ lost` | The daemon stopped unexpectedly (crash, `kill -9`, reboot). The processes are gone; only their details were kept. Press enter on one to relaunch it in the same directory, or `x` to forget it |
| "Observing" in the status bar, typing does nothing | Another window controls the session. `Ctrl+Space a` takes control |
| Terminating a shell takes a few seconds | Interactive shells ignore the polite quit signal, so Hive waits `[daemon] kill_timeout_secs` (default 3) before forcing it |
| `Kept worktree …: it has uncommitted changes` | Working as intended. Commit or stash, or `hive worktree rm <path> --force` |
| `hived did not come up` | See `hived.log`. Another `hived` may hold the lock; `hive daemon status` shows it |
| Socket errors with a custom `HIVE_RUNTIME_DIR` | Unix socket paths must be under ~107 bytes. Use a shorter directory; `hive doctor` warns about this |

## Running a private instance

To experiment without touching your real sessions, point Hive at scratch directories. It starts a separate daemon there:

```bash
export HIVE_RUNTIME_DIR=/tmp/hive-test HIVE_STATE_DIR=/tmp/hive-test/state HIVE_CONFIG=/tmp/hive-test/config.toml
hive new shell
hive daemon stop    # when done
```

## Reporting a bug

Include the output of `hive doctor`, your terminal emulator, and the relevant part of `hived.log` (with `HIVE_LOG=debug` if you can reproduce it). Check the log for anything private before sharing.

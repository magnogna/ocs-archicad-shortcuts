# PLUGIN.md — opencad.archicad_shortcuts

- Prefix: `AC_`. XDATA apps: none (the plugin stores no data).
- `dispatch` maps each command to a native command line and calls
  `HostApi::execute_command(line)`. The host's command-line driver starts the
  tool, stops at each `PAUSE` for user input, then consumes the queued answers.
- Before running, the plugin reads `host.selection()`. If empty it prints a
  hint and does nothing (otherwise the native tool would open its own
  "Select objects" prompt and the queued `PAUSE` tokens would answer the wrong
  prompts). The selection is never modified.
- No geometry, no undo, no selection logic lives in the plugin.

Why `execute_command` and not `run_command(CommandRequest)`: `run_command`
is a step-by-step script driver — after `Start` the caller must answer every
prompt itself (`Point`, `Text`, `Token`, …) and cannot hand a prompt to the
user. `execute_command` supports `PAUSE`, which is exactly "user answers this
prompt, then continue". Both share the same host guards (SCRIPTCOMMANDS,
denylist, plugin re-entry suppression).

# Archicad Shortcuts for Open CAD Studio

External, out-of-process plugin (`cdylib`, `ocs_plugin_api` API v10) that adds
Archicad (Windows) style editing commands to Open CAD Studio, plus a
`ArchicadWindows.cui` shortcut file. The host is not modified.

| Key | Command | Behaviour | Native command driven |
|---|---|---|---|
| Ctrl+D | `AC_MOVE` | Move selection | `MOVE` |
| Ctrl+Shift+D | `AC_COPYMOVE` | Original stays, one copy is placed | `COPY PAUSE PAUSE ENTER` |
| Ctrl+E | `AC_ROTATE` | Rotate selection | `ROTATE` |
| Ctrl+Shift+E | `AC_ROTATECOPY` | **Not implemented** — see [LIMITATIONS.md](LIMITATIONS.md) | — |
| Ctrl+M | `AC_MIRROR` | Mirror in place | `MIRROR PAUSE PAUSE Y` |
| Ctrl+Shift+M | `AC_MIRRORCOPY` | Original stays, mirrored copy | `MIRROR PAUSE PAUSE N` |
| Ctrl+G | `AC_GROUP` | Group selection using the host-generated name | `GROUP`, then Enter |
| Ctrl+Shift+G | `AC_UNGROUP` | Dissolve groups containing the selection | `UNGROUP` |

## Requirements

- Open CAD Studio with plugin API v10 (official release v2026.42, rustc 1.99.1), built with the same `rustc` as this
  plugin (the host checks `rustc_version` / `acadrust_source` in `plugin.toml`).
- Script commands enabled once at the OCS command line: `SCRIPTCOMMANDS 1`
  (the plugin drives native commands through `HostApi::execute_command`, which
  the host gates on this setting; the plugin never changes it).

## Install

1. Install from a GitHub Release (Plugin Manager) or copy the platform binary
   and `plugin.toml` into your OCS user plugins folder.
2. Apply the shortcuts manually: `CUIIMPORT <path>/ArchicadWindows.cui`.
   The plugin does not touch OCS configuration files.

After updating, import the new `.cui` again: installing the binary does not
update existing shortcut bindings. If Ctrl+Shift+M reports unknown command
`MIRRORCOPY`, the live binding is not `AC_MIRRORCOPY`. Re-import the supplied
file, or use the public host command `SHORTCUTS SET CTRL+SHIFT+M AC_MIRRORCOPY`.
Check Ctrl+M similarly (`SHORTCUTS SET CTRL+M AC_MIRROR`).

## Usage

Select objects first, then press the shortcut (or use the **Archicad** ribbon
tab). Points and angles are picked or typed exactly as in the native command;
undo is the native command's single undo step.

Ctrl+M queues **Yes to erase source**, which means **No to retain original**.
Do not change this answer to `N`: that would create a copy. A visible prompt
alone is not proof that the queued answer was consumed; the reported need to
answer manually has not been reproduced inside OCS and is not claimed fixed.

Ctrl+Shift+M still performs native mirror-copy; the newly requested literal
copy → move → mirror sequence is **not implemented**, because the current
public API cannot safely chain those interactive operations as one undo step.
See [LIMITATIONS.md](LIMITATIONS.md).

Ctrl+Shift+G uses UNGROUP, not geometric EXPLODE: entities stay intact. The host
dissolves every group containing any selected unlocked member, including
overlapping groups.

## Build

```sh
cargo test
cargo build --release
```

`Cargo.lock` is not committed yet; all git dependencies are pinned by `rev`,
so the resolved `acadrust` source is deterministic.

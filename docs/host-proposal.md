# Proposal for OpenCADStudio: copy + transform for plugins

Draft feature request (option C). Not submitted, no host code changed.
Goal: let `AC_ROTATECOPY` (copy → move → rotate) and `AC_MIRRORCOPY` (copy → move → mirror) run as
**one undo step** with native geometry, replacing the temporary chains.

## Options, from least to most invasive host change

1. **`Copy` option on ROTATE** (least invasive, ROTATECOPY only).
   In `RotateCommand`, a `C`/`Copy` keyword that returns
   `CmdResult::BatchCopy(handles, vec![EntityTransform::Rotate{..}])` instead of
   `TransformSelected`. ~20 lines in one file; reuses the existing BatchCopy
   handler (single undo). Same pattern MIRROR already uses with "Erase source".
   Does not cover the move step of copy → move → rotate/mirror, so it is
   no longer sufficient for the requested behaviour.
2. **Undo grouping for script lines** (small, solves both).
   An `UNDO BEGIN` / `UNDO END` (or `_UNDO G`) pair in the command driver that
   merges the undo entries committed between them into one. Plugin chains
   would become `UNDO BEGIN COPY ... ROTATE PAUSE PAUSE UNDO END`. Touches
   the history module and command parser; benefits all scripts, not only
   plugins. Needs care for a cancelled chain (END must still run).
3. **`HostApi::copy_transformed(handles, transforms)`** (largest, most general).
   New public API method + IPC message + API version bump, with a serialisable
   transform (translate/rotate/mirror/scale) backed by the existing
   `copy_entities` and one undo entry. Gives plugins full control, but the
   plugin still has to collect points itself (InteractiveCommand), and every
   API change requires plugins to be rebuilt for the new version.

## Recommendation

Option 1 is the smallest edit but no longer covers the requested copy → move →
rotate. Option 2 is the smallest change that fixes **both** commands and keeps all geometry and prompts native, so it
is the recommended request. Option 3 only if plugins need generic transforms.

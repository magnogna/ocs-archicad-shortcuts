# Limitations

## AC_ROTATECOPY (Ctrl+Shift+E) — not implemented

The command exists, is bound in `ArchicadWindows.cui`, and prints an
explanation; it does not modify the drawing.

Why it cannot be done correctly with the public API (OCS v2026.42, API v10):

1. **No native command.** `ROTATE` (RotateCommand) has only the `Reference`
   option; it always transforms the originals (`CmdResult::TransformSelected`).
   There is no `ROTATECOPY` and no Copy option.
2. **No public transform.** `EntityTransform::Rotate` and
   `Transformable::apply_transform` are host internals (`src/command`,
   `src/entities`), not part of `ocs_plugin_api`. Cloning entities from
   `document()` and rotating them in the plugin would duplicate host geometry
   per entity kind (arcs, text, dimensions, blocks, …) — rejected by design.
3. **Command chaining is not atomic.** e.g. `COPY` with zero displacement then
   `ROTATE` on the new entities: two undo steps; the plugin would have to
   change the user's selection to the copies and regain control after the
   user's input (execute_command returns before the user answers), so there
   is no reliable point to do it.

### What the host would need (not implemented here)

Any one of:

- a `Copy` option on `ROTATE` (like MIRROR's "Erase source objects?"), which
  would make `AC_ROTATECOPY` a one-line `ROTATE ... C ...` mapping; or
- a public `HostApi` method that copies and transforms handles with a
  serializable transform (translate/rotate/mirror) as one undo step.

## Other notes

### Requested copy → move → mirror — not implemented

`AC_MIRRORCOPY` retains its working native `MIRROR PAUSE PAUSE N` behavior.
It does not claim to implement the newly requested copy → move → mirror flow.
Native COPY/MOVE/MIRROR commit independently, clear selection on completion,
and `execute_command` returns before the user has supplied points. There is
no public command-chain transaction or completion callback that makes those
three interactive commands a reliable single undo with the copies selected.
Reimplementing entity transformations, polling the host or importing internals
would violate the plugin constraints. No host changes or workaround were made.

### Reported mirror prompt — not reproduced or claimed fixed

`AC_MIRROR` already queues `MIRROR PAUSE PAUSE Y`. In the pinned 2026.42 source,
the question is **Erase source objects?**, not **Keep original?**: `Y` erases
the original; `N` keeps it. Source inspection confirms the queued response,
but cannot establish why the tested installation still requires an answer.
Re-import the `.cui` to ensure Ctrl+M dispatches `AC_MIRROR`, not bare `MIRROR`.
If it still asks for an answer, capture the exact prompt and try `AC_MIRROR`
directly in the command line and from the ribbon to isolate the runtime issue.

### Keyboard bindings

The supplied `.cui` maps Ctrl+Shift+M to `AC_MIRRORCOPY`, not `MIRRORCOPY`.
Plugin installation does not apply bindings automatically. Re-import is
required after updates. No unprefixed alias is registered: the plugin claims
only `AC_` commands and never changes host settings itself.

### Group operations

`AC_GROUP` calls GROUP then ENTER, accepting the host-generated group name.
`AC_UNGROUP` calls UNGROUP, dissolving memberships rather than exploding
geometry. The host may dissolve multiple overlapping groups containing a
selected unlocked entity. Group creation/removal and undo remain native.

- All commands require `SCRIPTCOMMANDS 1` (host setting). If off, the plugin
  reports it and does nothing.
- `AC_ROTATE` runs bare `ROTATE`, so every native option (Reference) stays
  available.
- Not verified inside a running OCS instance: that no built-in Ctrl shortcut
  takes precedence over these CUI bindings, and the end-to-end PAUSE flow in
  the GUI. Verified by reading the host source, compiling the cdylib (both
  export symbols present) and unit tests of the command table.

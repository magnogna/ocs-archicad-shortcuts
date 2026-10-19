# Limitations

## AC_ROTATECOPY (Ctrl+Shift+E) — not implemented

The command exists, is bound in `ArchicadWindows.cui`, and prints an
explanation; it does not modify the drawing.

Why it cannot be done correctly with the public API (OCS commit 29bd657, API v13):

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

- All commands require `SCRIPTCOMMANDS 1` (host setting). If off, the plugin
  reports it and does nothing.
- `AC_ROTATE` runs bare `ROTATE`, so every native option (Reference) stays
  available.
- Not verified inside a running OCS instance: that no built-in Ctrl shortcut
  takes precedence over these CUI bindings, and the end-to-end PAUSE flow in
  the GUI. Verified by reading the host source, compiling the cdylib (both
  export symbols present) and unit tests of the command table.

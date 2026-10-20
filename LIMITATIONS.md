# Limitations

## Temporary command chains: AC_ROTATECOPY and AC_MIRRORCOPY

**These two commands are TEMPORARY.** They break the "one undo per operation"
rule, agreed as an interim solution. Remove them (see `TemporaryChain` in
`src/lib.rs`) once the host offers one of the options in
[docs/host-proposal.md](docs/host-proposal.md).

| Command | Chain (native commands only) | Result |
|---|---|---|
| `AC_ROTATECOPY` | `COPY PAUSE PAUSE ENTER ROTATE PAUSE PAUSE` | original stays, copy moved then rotated |
| `AC_MIRRORCOPY` | `COPY PAUSE PAUSE ENTER MIRROR PAUSE PAUSE Y` | original stays, copy moved then mirrored in place |

Why it works (host source): multiple-mode COPY returns `CopySelected`, whose
handler deselects the originals and selects the new copies; ending COPY with
ENTER does not clear the selection, so the next native command acts on the
copies. No geometry is computed by the plugin.

Known drawbacks:

- **Two undo steps** (COPY, then ROTATE/MIRROR). The plugin prints a notice.
- If the user cancels the second command, the copy stays. Undo once to remove it.
- The user's selection ends on the copies, not the originals.
- Not yet verified inside a running OCS 2026.42: PAUSE handover between two
  chained commands and the selection carry-over.

Why options A (atomic) and B were rejected: no public command-chain
transaction exists, and the public API exposes no entity transform; cloning
and transforming in the plugin would duplicate host geometry. `ARRAYPOLAR`
with 2 items was also evaluated for ROTATECOPY: one undo, but its angle is the
total (user would type 2× the angle) and its Associative toggle can only be
flipped blindly, so it is rejected as fragile.

## Other notes

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

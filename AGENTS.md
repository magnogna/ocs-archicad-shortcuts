# Plugin architecture

- Target the pinned official host public plugin API; never modify the host or import its internal modules, because this plugin must remain independently distributed.
- Drive native commands through HostApi::execute_command; use a separate ENTER call when accepting a text default, because inline ENTER is parsed as literal text by GROUP.
- Register only AC_ commands and distribute manual CUIIMPORT bindings; never rewrite host configuration, because shortcut installation is controlled by the user.
- Keep geometry, selection ownership and undo native; explicitly document unavailable interactive chains instead of implementing fragile workarounds.
- Run tests on feature/archicad-plugin; preserve main and published history, because plugin work belongs on its dedicated branch.

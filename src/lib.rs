//! Archicad (Windows) style shortcuts for Open CAD Studio.
//!
//! Every command drives a NATIVE host command through the public
//! `HostApi::execute_command`, which runs the line on the normal command-line
//! driver. `PAUSE` tokens hand the prompt to the user (viewport click or typed
//! value); fixed answers queued after them (`Y`, `N`, `ENTER`) are consumed by
//! the host once the user has answered. The plugin contains no geometry, no
//! selection system and no undo logic: the native command owns all of it.

use ocs_plugin_api::host::{BuiltinPlugin, HostApi};
use ocs_plugin_api::manifest::{ApiVersion, PluginManifest};
use ocs_plugin_api::ribbon::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};

// Keep in sync with plugin.toml.
static MANIFEST: PluginManifest = PluginManifest {
    id: "opencad.archicad_shortcuts",
    name: "Archicad Shortcuts",
    version: "0.1.0",
    description: "Archicad (Windows) style shortcuts driving native OCS commands.",
    api_version: ApiVersion::CURRENT,
    ribbon_order: 70,
    xdata_apps: &[],
    command_prefixes: &["AC_"],
};

/// What a plugin command does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Run this native command line.
    Native(&'static str),
    /// Deliberately not implemented; the message explains why.
    Unsupported(&'static str),
}

pub const ROTATECOPY_UNSUPPORTED: &str = "AC_ROTATECOPY is not available: OCS has no native \
rotate-a-copy command (ROTATE has no Copy option), and the public plugin API exposes no entity \
transform, so a correct single-undo copy+rotate cannot be built without host changes. See \
LIMITATIONS.md.";

/// Command table. Pure, so the mapping is unit-tested.
pub fn action_for(cmd: &str) -> Option<Action> {
    Some(match cmd.trim().to_ascii_uppercase().as_str() {
        // Native MOVE as-is: base point + destination, originals move.
        "AC_MOVE" => Action::Native("MOVE"),
        // Native COPY, limited to a single copy: base point, destination,
        // then Enter ends the multiple-copy loop. Originals stay.
        "AC_COPYMOVE" => Action::Native("COPY PAUSE PAUSE ENTER"),
        // Native ROTATE as-is: center + angle (Reference stays available).
        "AC_ROTATE" => Action::Native("ROTATE"),
        // Native MIRROR: two axis points, then "Erase source objects?" Yes.
        "AC_MIRROR" => Action::Native("MIRROR PAUSE PAUSE Y"),
        // Native MIRROR: two axis points, then "Erase source objects?" No.
        "AC_MIRRORCOPY" => Action::Native("MIRROR PAUSE PAUSE N"),
        "AC_ROTATECOPY" => Action::Unsupported(ROTATECOPY_UNSUPPORTED),
        _ => return None,
    })
}

struct ArchicadModule;

fn tool(id: &'static str, label: &'static str, glyph: &'static str) -> RibbonItem {
    RibbonItem::LargeTool(ToolDef {
        id,
        label,
        icon: IconKind::Glyph(glyph),
        event: ModuleEvent::Command(id.to_string()),
    })
}

impl CadModule for ArchicadModule {
    fn id(&self) -> &'static str {
        "archicad_shortcuts"
    }
    fn title(&self) -> &'static str {
        "Archicad"
    }
    fn ribbon_groups(&self) -> &[RibbonGroup] {
        static GROUPS: std::sync::OnceLock<Vec<RibbonGroup>> = std::sync::OnceLock::new();
        GROUPS.get_or_init(|| {
            vec![RibbonGroup {
                title: "Edit",
                tools: vec![
                    tool("AC_MOVE", "Move", "↔"),
                    tool("AC_COPYMOVE", "Copy Move", "⧉"),
                    tool("AC_ROTATE", "Rotate", "↻"),
                    tool("AC_ROTATECOPY", "Rotate Copy", "⟳"),
                    tool("AC_MIRROR", "Mirror", "⇋"),
                    tool("AC_MIRRORCOPY", "Mirror Copy", "⇌"),
                ],
            }]
        })
    }
}

struct ArchicadPlugin;

impl BuiltinPlugin for ArchicadPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn ribbon(&self) -> Box<dyn CadModule> {
        Box::new(ArchicadModule)
    }
    fn dispatch(&self, host: &mut dyn HostApi, cmd: &str) -> bool {
        let Some(action) = action_for(cmd) else {
            return false;
        };
        match action {
            Action::Unsupported(msg) => host.push_error(msg),
            Action::Native(line) => {
                // Use the host's current selection; never build our own.
                if host.selection().is_empty() {
                    host.push_info("Select objects first, then run the command again.");
                    return true;
                }
                if !host.execute_command(line) {
                    host.push_error(
                        "The host refused to run the command. Script commands may be off: \
                         enable them with SCRIPTCOMMANDS 1.",
                    );
                }
            }
        }
        true
    }
}

ocs_plugin_api::export_plugin!(ArchicadPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_runs_native_move() {
        assert_eq!(action_for("AC_MOVE"), Some(Action::Native("MOVE")));
    }
    #[test]
    fn copymove_makes_one_copy_with_native_copy() {
        assert_eq!(action_for("AC_COPYMOVE"), Some(Action::Native("COPY PAUSE PAUSE ENTER")));
    }
    #[test]
    fn rotate_runs_native_rotate() {
        assert_eq!(action_for("AC_ROTATE"), Some(Action::Native("ROTATE")));
    }
    #[test]
    fn mirror_erases_source() {
        assert_eq!(action_for("AC_MIRROR"), Some(Action::Native("MIRROR PAUSE PAUSE Y")));
    }
    #[test]
    fn mirrorcopy_keeps_source() {
        assert_eq!(action_for("AC_MIRRORCOPY"), Some(Action::Native("MIRROR PAUSE PAUSE N")));
    }
    #[test]
    fn rotatecopy_is_explicitly_unsupported() {
        assert!(matches!(action_for("AC_ROTATECOPY"), Some(Action::Unsupported(_))));
    }
    #[test]
    fn foreign_commands_are_not_claimed() {
        assert_eq!(action_for("MOVE"), None);
        assert_eq!(action_for("AC_UNKNOWN"), None);
    }
    #[test]
    fn manifest_matches_plugin_toml() {
        let toml = include_str!("../plugin.toml");
        assert!(toml.contains(&format!("id = \"{}\"", MANIFEST.id)));
        assert!(toml.contains("command_prefixes = [\"AC_\"]"));
        assert!(toml.contains(&format!("api_version = {}", ApiVersion::CURRENT.major)));
    }
    #[test]
    fn cui_binds_only_plugin_commands_in_cuiimport_format() {
        let cui = include_str!("../ArchicadWindows.cui");
        let rows: Vec<(&str, &str)> = cui
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.split_once(char::is_whitespace).unwrap())
            .map(|(k, v)| (k, v.trim()))
            .collect();
        assert_eq!(rows.len(), 6);
        for (k, v) in rows {
            assert!(k.starts_with("CTRL+"), "{k}");
            assert!(action_for(v).is_some(), "{v}");
        }
    }
}

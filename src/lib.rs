//! Archicad (Windows) style shortcuts for Open CAD Studio.
//!
//! Every command drives a NATIVE host command through the public
//! `HostApi::execute_command`, which runs the line on the normal command-line
//! driver. `PAUSE` tokens hand the prompt to the user (viewport click or typed
//! value); fixed answers queued after them (`Y`, `N`, `ENTER`) are consumed by
//! the host once the user has answered. The plugin contains no geometry, no
//! selection system and no undo logic: the native command owns all of it.

use ocs_plugin_api::host::{BuiltinPlugin, HostApi, HostNotification, PluginRequestSender};
use ocs_plugin_api::ipc::protocol::{PluginRequest, PluginResponse};
use ocs_plugin_api::host::Handle;
use std::sync::{Arc, Mutex};
use ocs_plugin_api::manifest::{ApiVersion, PluginManifest};
use ocs_plugin_api::ribbon::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};

// Keep in sync with plugin.toml.
static MANIFEST: PluginManifest = PluginManifest {
    id: "opencad.archicad_shortcuts",
    name: "Archicad Shortcuts",
    version: "0.2.1",
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
    /// Start a native command, then accept its default with Enter.
    NativeThenEnter(&'static str),
    /// TEMPORARY chain of native commands (two undo steps). See LIMITATIONS.md.
    /// (first line, follow-up line): the follow-up starts only after the
    /// first native command has ended (see `PendingChain`).
    TemporaryChain(&'static str, &'static str),
}

/// TEMPORARY (see LIMITATIONS.md, "Temporary command chains"): OCS has no
/// native rotate-a-copy. Chain: copy -> move -> rotate. Native COPY places one
/// copy (the host selects it), then native ROTATE on that selection.
/// Produces TWO undo steps. Remove once the host offers an atomic copy+transform.
/// The host feeds queued tokens only to the command still running, so a
/// second verb in the same line is dropped when COPY ends: the follow-up is
/// sent separately once the host reports that COPY has finished.
pub const COPY_STEP: &str = "COPY PAUSE PAUSE ENTER";
pub const ROTATECOPY_FOLLOWUP: &str = "ROTATE PAUSE PAUSE";
/// TEMPORARY: copy -> move -> mirror. Native COPY places one copy (the host
/// selects it), then native MIRROR erases its source, i.e. mirrors the copy
/// in place. Originals stay. Produces TWO undo steps.
pub const MIRRORCOPY_FOLLOWUP: &str = "MIRROR PAUSE PAUSE Y";
pub const TEMPORARY_NOTICE: &str = "Temporary implementation: this command creates two undo \
steps (copy, then transform). Undo twice to revert it completely.";

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
        // TEMPORARY: copy -> move -> mirror (two undo steps).
        "AC_MIRRORCOPY" => Action::TemporaryChain(COPY_STEP, MIRRORCOPY_FOLLOWUP),
        // Accept GROUP's host-generated name, without naming the group ENTER.
        "AC_GROUP" => Action::NativeThenEnter("GROUP"),
        // UNGROUP dissolves membership; EXPLODE changes entity geometry.
        "AC_UNGROUP" => Action::Native("UNGROUP"),
        // TEMPORARY: copy -> move -> rotate (two undo steps).
        "AC_ROTATECOPY" => Action::TemporaryChain(COPY_STEP, ROTATECOPY_FOLLOWUP),
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
                    tool("AC_GROUP", "Group", "▣"),
                    tool("AC_UNGROUP", "Ungroup", "▦"),
                ],
            }]
        })
    }
}

/// TEMPORARY chain bookkeeping: waits for COPY to start and then end, and
/// only then sends the follow-up, if COPY really produced new copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingChain {
    pub followup: &'static str,
    pub originals: Vec<Handle>,
    pub started: bool,
}

/// Feed one "active command" change. Returns true when the first command has
/// ended and the follow-up should be evaluated.
pub fn chain_step(pending: &mut PendingChain, command: Option<&str>) -> bool {
    match command {
        Some(_) => {
            pending.started = true;
            false
        }
        None => pending.started,
    }
}

/// COPY selects the copies it placed. An empty selection, or the original
/// one, means it was cancelled: then no follow-up runs.
pub fn copies_were_made(originals: &[Handle], after: &[Handle]) -> bool {
    !after.is_empty() && !after.iter().all(|h| originals.contains(h))
}

#[derive(Default)]
struct ArchicadPlugin {
    sender: Mutex<Option<Arc<dyn PluginRequestSender>>>,
    pending: Mutex<Option<PendingChain>>,
}

impl ArchicadPlugin {
    fn run_followup(&self, chain: PendingChain) {
        let Some(sender) = self.sender.lock().unwrap().clone() else {
            return;
        };
        // Off the runner thread: the request waits for the host's reply.
        std::thread::spawn(move || {
            let after = match sender.request(PluginRequest::GetSelection) {
                Ok(PluginResponse::Selection(h)) | Ok(PluginResponse::Handles(h)) => h,
                _ => return,
            };
            if !copies_were_made(&chain.originals, &after) {
                return;
            }
            let _ = sender.request(PluginRequest::ExecuteCommand(chain.followup.to_string()));
        });
    }
}

struct SenderBox(Box<dyn PluginRequestSender>);
impl PluginRequestSender for SenderBox {
    fn request(
        &self,
        req: PluginRequest,
    ) -> Result<PluginResponse, ocs_plugin_api::host::PluginRequestError> {
        self.0.request(req)
    }
}

impl BuiltinPlugin for ArchicadPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn ribbon(&self) -> Box<dyn CadModule> {
        Box::new(ArchicadModule)
    }
    fn on_load(&mut self, host: &mut dyn HostApi) {
        *self.sender.lock().unwrap() =
            host.plugin_request_sender().map(|s| Arc::new(SenderBox(s)) as Arc<dyn PluginRequestSender>);
    }
    fn on_notification(&mut self, _command_id: Option<u64>, notification: HostNotification) {
        let HostNotification::CommandStateChanged { command, .. } = notification else {
            return;
        };
        let ready = {
            let mut guard = self.pending.lock().unwrap();
            let done = guard.as_mut().is_some_and(|p| chain_step(p, command.as_deref()));
            if done { guard.take() } else { None }
        };
        if let Some(chain) = ready {
            self.run_followup(chain);
        }
    }
    fn dispatch(&self, host: &mut dyn HostApi, cmd: &str) -> bool {
        let Some(action) = action_for(cmd) else {
            return false;
        };
        // Use the host's current selection; never build our own.
        let originals = host.selection();
        if originals.is_empty() {
            host.push_info("Select objects first, then run the command again.");
            return true;
        }
        let line = match action {
            Action::Native(l) | Action::NativeThenEnter(l) | Action::TemporaryChain(l, _) => l,
        };
        if let Action::TemporaryChain(_, followup) = action {
            if self.sender.lock().unwrap().is_none() {
                host.push_error("This host does not allow the plugin to chain commands.");
                return true;
            }
            *self.pending.lock().unwrap() =
                Some(PendingChain { followup, originals: originals.clone(), started: false });
        }
        if !host.execute_command(line) {
            *self.pending.lock().unwrap() = None;
            host.push_error(
                "The host refused to run the command. Script commands may be off: \
                 enable them with SCRIPTCOMMANDS 1.",
            );
        } else if matches!(action, Action::NativeThenEnter(_)) && !host.execute_command("ENTER") {
            host.push_error("The host could not accept the default group name.");
        } else if matches!(action, Action::TemporaryChain(..)) {
            host.push_info(TEMPORARY_NOTICE);
        }
        true
    }
}

ocs_plugin_api::export_plugin!(ArchicadPlugin::default());

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
        // copy -> move -> mirror: single native copy, then mirror erasing the
        // copy's source (the copy itself), never the originals.
        assert_eq!(
            action_for("AC_MIRRORCOPY"),
            Some(Action::TemporaryChain("COPY PAUSE PAUSE ENTER", "MIRROR PAUSE PAUSE Y"))
        );
    }
    #[test]
    fn rotatecopy_copies_moves_then_rotates_copy() {
        // copy -> move -> rotate: single native copy, then rotate the selected copy.
        assert_eq!(
            action_for("AC_ROTATECOPY"),
            Some(Action::TemporaryChain("COPY PAUSE PAUSE ENTER", "ROTATE PAUSE PAUSE"))
        );
    }
    #[test]
    fn group_uses_native_group_and_accepts_default_name() {
        assert_eq!(action_for("AC_GROUP"), Some(Action::NativeThenEnter("GROUP")));
    }
    #[test]
    fn ungroup_dissolves_membership_without_exploding_geometry() {
        assert_eq!(action_for("AC_UNGROUP"), Some(Action::Native("UNGROUP")));
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
        assert_eq!(rows.len(), 8);
        for (k, v) in rows {
            assert!(k.starts_with("CTRL+"), "{k}");
            assert!(action_for(v).is_some(), "{v}");
        }
    }
    fn binding(key: &str) -> Option<&'static str> {
        include_str!("../ArchicadWindows.cui").lines()
            .filter(|line| !line.trim().starts_with('#'))
            .filter_map(|line| line.split_once(char::is_whitespace))
            .find(|(bound, _)| *bound == key)
            .map(|(_, command)| command.trim())
    }
    #[test]
    fn ctrl_m_routes_to_mirror_erasing_originals() {
        assert_eq!(binding("CTRL+M"), Some("AC_MIRROR"));
    }
    #[test]
    fn ctrl_shift_m_routes_to_plugin_not_unknown_mirrorcopy() {
        assert_eq!(binding("CTRL+SHIFT+M"), Some("AC_MIRRORCOPY"));
    }
    #[test]
    fn ctrl_g_groups() {
        assert_eq!(binding("CTRL+G"), Some("AC_GROUP"));
    }
    #[test]
    fn ctrl_shift_g_ungroups() {
        assert_eq!(binding("CTRL+SHIFT+G"), Some("AC_UNGROUP"));
    }

    #[test]
    fn followup_waits_for_copy_to_start_and_end() {
        let mut p = PendingChain { followup: "ROTATE PAUSE PAUSE", originals: vec![], started: false };
        assert!(!chain_step(&mut p, None));
        assert!(!chain_step(&mut p, Some("COPY")));
        assert!(chain_step(&mut p, None));
    }

    #[test]
    fn followup_skipped_when_copy_cancelled() {
        let a = Handle::new(1);
        assert!(!copies_were_made(&[a], &[a]));
        assert!(!copies_were_made(&[a], &[]));
        assert!(copies_were_made(&[a], &[Handle::new(2)]));
    }
}

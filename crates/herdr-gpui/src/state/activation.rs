use crate::{NavigationTarget, OwnedNavigationTarget};

#[derive(Clone, Copy, Default)]
pub(crate) enum ActivationAck {
    #[default]
    Surface,
    /// A focus change advances snapshot metadata. Its own acknowledgement,
    /// followed by that coherent projection, replaces a second API barrier.
    Focus { after_revision: u64 },
}

#[derive(Clone)]
pub struct SurfaceActivation {
    pub request: String,
    pub(crate) ack: ActivationAck,
    pub boot: String,
    pub revision: Option<u64>,
    pub failed: bool,
    pub focus: Option<OwnedNavigationTarget>,
    pub active: bool,
}

impl SurfaceActivation {
    pub(super) fn acknowledged_revision(&self, response: &serde_json::Value) -> Option<u64> {
        if self.failed || response.get("error").is_some_and(|error| !error.is_null()) {
            return None;
        }
        let result = &response["result"];
        match self.ack {
            ActivationAck::Surface => (result["type"] == "client_shell_surface_set"
                && result["active"] == self.active)
                .then(|| result["projection_revision"].as_u64())
                .flatten(),
            ActivationAck::Focus { after_revision } => {
                let (kind, object, field, target) = match self.focus.as_ref()? {
                    NavigationTarget::Pane(id) => ("pane_info", "pane", "pane_id", id),
                    NavigationTarget::Tab(id) => ("tab_info", "tab", "tab_id", id),
                    NavigationTarget::Workspace(id) => {
                        ("workspace_info", "workspace", "workspace_id", id)
                    }
                };
                (self.active && result["type"] == kind && result[object][field] == *target)
                    .then(|| after_revision.checked_add(1))
                    .flatten()
            }
        }
    }
}

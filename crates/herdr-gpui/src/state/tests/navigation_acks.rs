use super::*;

fn navigation(
    target: crate::OwnedNavigationTarget,
    old: &ClientShellSnapshot,
) -> SurfaceActivation {
    SurfaceActivation {
        request: "focus-request".into(),
        ack: ActivationAck::Focus {
            after_revision: old.revision,
        },
        boot: old.boot_id.clone(),
        revision: None,
        failed: false,
        focus: Some(target),
        active: true,
    }
}

fn response(kind: &str, object: &str, field: &str, id: &str) -> ClientEvent {
    ClientEvent::Response {
        request_id: "focus-request".into(),
        response: serde_json::json!({"result": {"type": kind, object: {field: id}}}),
    }
}

#[test]
fn focus_ack_requires_new_coherent_target_projection_in_either_arrival_order() {
    for (target, kind, object, field) in [
        (
            crate::NavigationTarget::<String>::Pane("destination".into()),
            "pane_info",
            "pane",
            "pane_id",
        ),
        (
            crate::NavigationTarget::Tab("destination".into()),
            "tab_info",
            "tab",
            "tab_id",
        ),
        (
            crate::NavigationTarget::Workspace("destination".into()),
            "workspace_info",
            "workspace",
            "workspace_id",
        ),
    ] {
        for ack_first in [false, true] {
            let old = snapshot();
            let mut next = (*old).clone();
            next.revision += 1;
            match &target {
                crate::NavigationTarget::Pane(id) => next.focused_pane_id = Some(id.clone()),
                crate::NavigationTarget::Tab(id) => next.focused_tab_id = Some(id.clone()),
                crate::NavigationTarget::Workspace(id) => {
                    next.focused_workspace_id = Some(id.clone())
                }
            }
            let mut state = LiveState::default();
            state.apply(ClientEvent::Snapshot(old.clone()));
            state.activation = Some(navigation(target.clone(), &old));
            if ack_first {
                state.apply(response(kind, object, field, "destination"));
                assert!(!state.surface_ready());
            }
            state.apply(ClientEvent::Snapshot(Arc::new(next.clone())));
            assert!(!state.surface_ready());
            state.apply(ClientEvent::Surface(surface(&next)));
            assert_eq!(state.surface_ready(), ack_first);
            if !ack_first {
                state.apply(response(kind, object, field, "destination"));
            }
            assert!(state.surface_ready());
            assert!(state.activation.as_ref().unwrap().focus.is_none());
        }
    }
}

#[test]
fn navigation_never_opens_for_unrelated_reply_old_revision_or_wrong_focus() {
    let old = snapshot();
    let mut state = LiveState::default();
    state.apply(ClientEvent::Snapshot(old.clone()));
    state.activation = Some(navigation(
        crate::NavigationTarget::Pane("destination".into()),
        &old,
    ));
    state.apply(ClientEvent::Response {
        request_id: "unrelated".into(),
        response: serde_json::json!({"result": {"type": "pane_info", "pane": {"pane_id": "destination"}}}),
    });
    assert!(state.activation.as_ref().unwrap().revision.is_none());
    state.apply(response("pane_info", "pane", "pane_id", "destination"));
    let mut next = (*old).clone();
    next.focused_pane_id = Some("destination".into());
    state.apply(ClientEvent::Snapshot(Arc::new(next.clone())));
    state.apply(ClientEvent::Surface(surface(&next)));
    assert!(
        !state.surface_ready(),
        "same revision cannot prove a focus change"
    );
    next.revision += 1;
    next.focused_pane_id = Some("different".into());
    state.apply(ClientEvent::Snapshot(Arc::new(next.clone())));
    state.apply(ClientEvent::Surface(surface(&next)));
    assert!(
        !state.surface_ready(),
        "new revision still needs the requested target"
    );
    next.revision += 1;
    next.focused_pane_id = Some("destination".into());
    state.apply(ClientEvent::Snapshot(Arc::new(next.clone())));
    assert!(!state.surface_ready());
    state.apply(ClientEvent::Surface(surface(&next)));
    assert!(state.surface_ready());
}

#[test]
fn failed_wrong_target_malformed_and_rebooted_navigation_stays_fenced() {
    let old = snapshot();
    for event in [
        ClientEvent::CommandRejected {
            request_id: Some("focus-request".into()),
            reason: herdr_client::Error::UnsupportedMethod,
        },
        ClientEvent::Response {
            request_id: "focus-request".into(),
            response: serde_json::json!({"error": {"code": "pane_not_found"}}),
        },
        response("tab_info", "tab", "tab_id", "destination"),
        response("pane_info", "pane", "pane_id", "wrong"),
        ClientEvent::Response {
            request_id: "focus-request".into(),
            response: serde_json::json!({"result": {}}),
        },
    ] {
        let mut state = LiveState::default();
        state.apply(ClientEvent::Snapshot(old.clone()));
        state.activation = Some(navigation(
            crate::NavigationTarget::Pane("destination".into()),
            &old,
        ));
        state.apply(event);
        assert!(state.activation.as_ref().unwrap().failed);
        assert!(!state.surface_ready());
    }
    let mut state = LiveState::default();
    state.apply(ClientEvent::Snapshot(old.clone()));
    state.activation = Some(navigation(
        crate::NavigationTarget::Pane("destination".into()),
        &old,
    ));
    let mut reboot = (*old).clone();
    reboot.boot_id = "reboot".into();
    reboot.revision += 1;
    reboot.focused_pane_id = Some("destination".into());
    state.apply(ClientEvent::Snapshot(Arc::new(reboot.clone())));
    state.apply(response("pane_info", "pane", "pane_id", "destination"));
    state.apply(ClientEvent::Surface(surface(&reboot)));
    assert!(!state.surface_ready());
}

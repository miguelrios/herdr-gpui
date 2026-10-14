use super::*;

#[test]
fn semantic_input_captures_original_explicit_destination_before_queueing() {
    let (commands, receiver) = queue::channel(4).unwrap();
    let handle = ClientHandle {
        inner: Arc::new(HandleInner {
            commands,
            stop: Arc::new(AtomicBool::new(false)),
            next_request: AtomicU64::new(1),
            image_busy: Arc::new(AtomicBool::new(false)),
            last_queued_theme: Default::default(),
            liveness: Arc::new(AtomicBool::new(false)),
        }),
    };
    handle
        .send_input(
            "boot-v1",
            "w1:p1",
            [ClientPaneInputEvent::TextCommit("public".into())],
        )
        .unwrap();
    handle
        .send_popup_input(
            "boot-v1",
            "popup-public",
            [ClientPaneInputEvent::TextCommit("public".into())],
        )
        .unwrap();
    handle.resize("boot-v1", ConnectOptions::default()).unwrap();
    handle.focus_pane("boot-v1", "w1:p1").unwrap();
    for expected in [
        Some(InputTarget::Pane("w1:p1".into())),
        Some(InputTarget::Popup("popup-public".into())),
        None,
        None,
    ] {
        let command = receiver.try_recv().unwrap();
        assert_eq!(command.input_target, expected);
        let message: ClientMessage = decode_payload(&command.bytes[4..]).unwrap();
        match (expected, message) {
            (Some(InputTarget::Pane(id)), ClientMessage::ClientShellPaneInput { pane_id, .. }) => {
                assert_eq!(id, pane_id)
            }
            (
                Some(InputTarget::Popup(id)),
                ClientMessage::ClientShellPopupInput { terminal_id, .. },
            ) => assert_eq!(id, terminal_id),
            (
                None,
                ClientMessage::ClientShellResize { .. }
                | ClientMessage::ClientShellEndpointRequest { .. },
            ) => {}
            other => panic!("unexpected original target/message: {other:?}"),
        }
    }
}

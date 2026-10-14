use super::*;

fn respond_focus(server: &mut Server, request: &serde_json::Value) {
    let id = request["id"].as_str().unwrap();
    write_message(
        &mut server.stream,
        &ServerMessage::ClientShellEndpointResponseChunk {
            boot_id: snapshot().boot_id,
            request_id: id.into(),
            final_chunk: true,
            data: serde_json::to_vec(&serde_json::json!({
                "id": id,
                "result": {"type": "pane_info", "pane": {"pane_id": "destination"}}
            }))
            .unwrap(),
        },
        MAX_GRAPHICS_FRAME_SIZE,
    )
    .unwrap();
}

#[gpui::test]
fn changed_pane_uses_one_api_request_and_waits_for_ack_and_new_surface(
    cx: &mut gpui::TestAppContext,
) {
    for ack_first in [false, true] {
        let (endpoint, mut server) = connected_endpoint("navigation-ack");
        let (fixture, cx) = cx.add_window_view(|window, cx| {
            Fixture(cx.new(|cx| crate::sidebar::layout_tests::fixture_window(window, cx)))
        });
        let view = fixture.read_with(cx, |fixture, _| fixture.0.clone());
        view.update(cx, |view, cx| {
            view.endpoints = vec![endpoint];
            view.options = ConnectOptions::default();
            view.reset_selected(cx);
            assert!(view.input_ready());
            assert!(view.navigate(NavigationTarget::Pane("destination"), cx));
            assert!(!view.input_ready());
            view.send(
                ClientPaneInputEvent::TextCommit("discard before focus".into()),
                cx,
            );
            view.endpoints[0]
                .connection
                .handle
                .as_ref()
                .unwrap()
                .set_focus(&snapshot().boot_id, false)
                .unwrap();
        });
        let ClientMessage::ClientShellEndpointRequest { request, .. } = server.receive() else {
            panic!("missing focus request");
        };
        let request: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(request["method"], "pane.focus");
        // The marker is behind navigation. A queued second API barrier would
        // block it behind the unacknowledged first request.
        assert!(matches!(
            server.receive(),
            ClientMessage::ClientShellFocus { focused: false }
        ));
        if ack_first {
            respond_focus(&mut server, &request);
            view.update(cx, |view, cx| {
                project_until(view, cx, "focus acknowledgement", |view| {
                    view.live.activation.as_ref().unwrap().revision.is_some()
                });
                assert!(!view.input_ready());
            });
        }
        let mut next = snapshot();
        next.revision += 1;
        next.focused_pane_id = Some("destination".into());
        view.update(cx, |view, cx| {
            {
                let mut inbox = view.endpoints[0].connection.inbox.lock().unwrap();
                inbox.apply(ClientEvent::Snapshot(Arc::new(next.clone())));
                inbox.apply(ClientEvent::Surface(surface(&next)));
            }
            project_until(view, cx, "target projection", |view| {
                view.live.snapshot.as_ref().unwrap().revision == next.revision
            });
            assert_eq!(view.input_ready(), ack_first);
        });
        if !ack_first {
            respond_focus(&mut server, &request);
        }
        view.update(cx, |view, cx| {
            project_until(view, cx, "navigation completed", HerdrWindow::input_ready);
            assert!(view.activation_deadline.is_none());
            view.send(
                ClientPaneInputEvent::TextCommit("destination only".into()),
                cx,
            );
        });
        let ClientMessage::ClientShellPaneInput { pane_id, .. } = server.receive() else {
            panic!("missing input after coherent focus");
        };
        assert_eq!(pane_id, "destination");
    }
}

#[gpui::test]
fn same_pane_and_popup_navigation_keep_the_surface_epoch_barrier(cx: &mut gpui::TestAppContext) {
    for popup in [false, true] {
        let (endpoint, mut server) = connected_endpoint("navigation-fallback");
        let (fixture, cx) = cx.add_window_view(|window, cx| {
            Fixture(cx.new(|cx| crate::sidebar::layout_tests::fixture_window(window, cx)))
        });
        let view = fixture.read_with(cx, |fixture, _| fixture.0.clone());
        view.update(cx, |view, cx| {
            view.endpoints = vec![endpoint];
            view.options = ConnectOptions::default();
            view.reset_selected(cx);
            if popup {
                let mut frame = (*view.live.surface.as_ref().unwrap().as_ref()).clone();
                frame.popup = Some(Box::new(ClientShellPopupSurface {
                    terminal_id: "popup".into(),
                    title: "popup".into(),
                    width: None,
                    height: None,
                    mouse_reporting: false,
                    sgr_pixel_mouse: false,
                    pixel_width: 0,
                    pixel_height: 0,
                    frame: frame.frame.clone(),
                }));
                let frame = Arc::new(frame);
                view.live.surface = Some(frame.clone());
                view.endpoints[0].connection.inbox.lock().unwrap().surface = Some(frame);
            }
            let target = if popup {
                "destination".into()
            } else {
                view.live
                    .snapshot
                    .as_ref()
                    .unwrap()
                    .focused_pane_id
                    .clone()
                    .unwrap()
            };
            assert!(view.navigate(NavigationTarget::Pane(&target), cx));
            assert!(matches!(
                view.live.activation.as_ref().unwrap().ack,
                crate::state::ActivationAck::Surface
            ));
        });
        let ClientMessage::ClientShellEndpointRequest { request, .. } = server.receive() else {
            panic!("missing focus request");
        };
        let request: serde_json::Value = serde_json::from_str(&request).unwrap();
        server.respond(&request);
        let ClientMessage::ClientShellEndpointRequest { request, .. } = server.receive() else {
            panic!("missing surface barrier");
        };
        let barrier: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(barrier["method"], "client_shell.surface.set");
    }
}

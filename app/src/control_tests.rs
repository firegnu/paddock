use super::*;
use std::path::Path;

fn directory() -> PathBuf {
    let path = PathBuf::from("/tmp").join(format!("pc-{}", random_id().unwrap()));
    fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
    path
}
fn message(id: &str, request: Option<&str>, operation: Operation) -> Message {
    Message {
        instance: id.into(),
        request_id: request.map(str::to_owned),
        caller: Caller::default(),
        operation,
    }
}
fn browse() -> Operation {
    Operation::Browse {
        url: "http://localhost:3000".into(),
        focus: false,
    }
}
fn round_trip(server: &mut Server, operation: Operation, request: Option<&str>) -> Value {
    let message = message(&server.id, request, operation);
    let directory = server.path.parent().unwrap().to_owned();
    let client = thread::spawn(move || exchange_in(&directory, &message).unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !client.is_finished() {
        assert!(Instant::now() < deadline);
        server.process_pending(unsupported);
        thread::sleep(Duration::from_millis(5));
    }
    client.join().unwrap()
}
#[test]
fn directory_and_socket_permissions_links_and_length_are_checked() {
    let dir = directory();
    let mut server = Server::start_in(&dir).unwrap();
    assert_eq!(fs::metadata(&dir).unwrap().mode() & 0o7777, 0o700);
    let metadata = fs::symlink_metadata(&server.path).unwrap();
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    assert_eq!(
        round_trip(&mut server, Operation::Inspect, None)["error"]["code"],
        "unsupported"
    );
    fs::set_permissions(&server.path, fs::Permissions::from_mode(0o666)).unwrap();
    let msg = message(&server.id, None, Operation::Instances);
    assert!(
        exchange_in(&dir, &msg)
            .unwrap_err()
            .to_string()
            .contains("0600")
    );
    fs::set_permissions(&server.path, fs::Permissions::from_mode(0o600)).unwrap();
    let link = dir.join("linked");
    std::os::unix::fs::symlink(&dir, &link).unwrap();
    assert!(Server::start_in(&link).is_err());
    let file = dir.join("file");
    fs::write(&file, b"occupied").unwrap();
    assert!(Server::start_in(&file).is_err());
    let long = dir.join("x".repeat(100));
    assert!(
        Server::start_in(&long)
            .err()
            .unwrap()
            .to_string()
            .contains("too long")
    );
    for invalid in ["../escape", "", "zzzzzzzzzzzzzzzz"] {
        assert!(socket_path(&dir, invalid).is_err());
    }
    // A foreign-owned directory must be refused, never chmod'ed. Use OS metadata
    // rather than chown (the test does not require elevated privileges).
    let root = fs::symlink_metadata(Path::new("/")).unwrap();
    if root.uid() != unsafe { libc::geteuid() } {
        assert!(
            private_directory(&root)
                .unwrap_err()
                .to_string()
                .contains("owned")
        );
    }
}
#[test]
fn two_instances_stale_registration_and_drop_cleanup() {
    let dir = directory();
    let first = Server::start_in(&dir).unwrap();
    let second = Server::start_in(&dir).unwrap();
    assert_ne!(first.id, second.id);
    let stale = socket_path(&dir, "aaaaaaaaaaaaaaaa").unwrap();
    drop(UnixListener::bind(&stale).unwrap());
    fs::set_permissions(&stale, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(dir.join("unrelated.txt"), b"keep").unwrap();
    let live = instances_in(&dir, &Caller::default()).unwrap();
    assert_eq!(live.len(), 2);
    assert!(live.iter().any(|v| v["instance"] == first.id));
    assert!(live.iter().any(|v| v["instance"] == second.id));
    let first_path = first.path.clone();
    drop(first);
    assert!(!first_path.exists());
    assert!(second.path.exists());
    assert_eq!(instances_in(&dir, &Caller::default()).unwrap().len(), 1);
    let second_path = second.path.clone();
    drop(second);
    assert!(!second_path.exists());
    assert!(instances_in(&dir, &Caller::default()).unwrap().is_empty());
    assert_eq!(fs::read(dir.join("unrelated.txt")).unwrap(), b"keep");
}
#[test]
fn malformed_oversize_incomplete_and_unknown_messages_are_rejected() {
    let dir = directory();
    let server = Server::start_in(&dir).unwrap();
    for bytes in [
        b"not-json\n".to_vec(),
        b"{}\n".to_vec(),
        vec![b'x'; LIMIT + 1],
    ] {
        let mut stream = UnixStream::connect(&server.path).unwrap();
        stream.write_all(&bytes).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        assert_eq!(
            read_json(&mut stream).unwrap()["error"]["code"],
            "invalid_request"
        );
    }
    let mut wire = serde_json::to_value(message(&server.id, None, Operation::Inspect)).unwrap();
    wire["extra"] = json!(true);
    let mut stream = UnixStream::connect(&server.path).unwrap();
    write_json(&mut stream, &wire).unwrap();
    assert_eq!(
        read_json(&mut stream).unwrap()["error"]["code"],
        "invalid_request"
    );
    let mut stream = UnixStream::connect(&server.path).unwrap();
    write_json(
        &mut stream,
        &message("bbbbbbbbbbbbbbbb", None, Operation::Inspect),
    )
    .unwrap();
    assert_eq!(
        read_json(&mut stream).unwrap()["error"]["code"],
        "instance_unavailable"
    );
    let (mut stream, _peer) = UnixStream::pair().unwrap();
    assert!(write_json(&mut stream, &json!("x".repeat(LIMIT))).is_err());
}
#[test]
fn slow_incomplete_message_hits_total_read_deadline() {
    let (mut stream, _peer) = UnixStream::pair().unwrap();
    let start = Instant::now();
    assert!(
        read_json(&mut stream)
            .unwrap_err()
            .to_string()
            .contains("timed out")
    );
    assert!(start.elapsed() >= Duration::from_millis(1900));
    assert!(start.elapsed() < Duration::from_secs(8));
}
#[test]
fn timeout_keeps_generated_request_queryable_and_mutation_executes_once() {
    let dir = directory();
    let mut server = Server::start_in(&dir).unwrap();
    // No UI pumping until after the transport's reply deadline.
    let result = exchange_in(&dir, &message(&server.id, None, browse())).unwrap();
    assert_eq!(result["state"], "uncertain");
    let id = result["request_id"].as_str().unwrap();
    let mut calls = 0;
    server.process_pending(|_, _| {
        calls += 1;
        json!({"ok":true,"state":"starting","pane":3,"revision":1})
    });
    assert_eq!(calls, 1);
    let value = round_trip(&mut server, Operation::Request { request: id.into() }, None);
    assert_eq!(value["state"], "starting");
    assert_eq!(value["request_id"], id);
    assert_eq!(round_trip(&mut server, browse(), Some(id)), value);
    assert!(server.records.update(
        id,
        json!({"ok":true,"state":"complete","pane":3,"revision":1})
    ));
    let complete = round_trip(&mut server, Operation::Request { request: id.into() }, None);
    assert_eq!(complete["state"], "complete");
    assert_eq!(complete["instance"], server.id);
}
#[test]
fn ui_queue_is_bounded_and_full_queue_returns_busy_with_identity() {
    let (tx, rx) = mpsc::sync_channel(32);
    for n in 0..32 {
        let (reply, _wait) = mpsc::sync_channel(1);
        tx.try_send(Incoming {
            message: message("0123456789abcdef", Some(&n.to_string()), browse()),
            reply,
        })
        .unwrap();
    }
    let full = enqueue(&tx, message("0123456789abcdef", Some("overflow"), browse()));
    assert_eq!(full["error"]["code"], "busy");
    assert_eq!(full["request_id"], "overflow");
    for n in 0..32 {
        assert_eq!(
            rx.try_recv().unwrap().message.request_id.unwrap(),
            n.to_string()
        );
    }
    assert!(rx.try_recv().is_err());
}
#[test]
fn records_capacity_replay_updates_and_busy_do_not_lose_results() {
    let mut records = Records::default();
    let mut calls = 0;
    for n in 0..256 {
        let msg = message("0123456789abcdef", Some(&n.to_string()), browse());
        let value = records.dispatch(&msg, &mut |_, _| {
            calls += 1;
            json!({"ok":true,"state":"accepted"})
        });
        assert_eq!(value["state"], "accepted");
    }
    let msg = message("0123456789abcdef", Some("256"), browse());
    assert_eq!(
        records.dispatch(&msg, &mut |_, _| panic!("must not execute"))["error"]["code"],
        "request_limit"
    );
    assert_eq!(calls, 256);
    for state in [
        "starting",
        "complete",
        "failed",
        "target_invalid",
        "uncertain",
    ] {
        assert!(records.update("0", json!({"ok":true,"state":state})));
        let replay = records.dispatch(
            &message("0123456789abcdef", Some("0"), browse()),
            &mut |_, _| panic!("must not replay"),
        );
        assert_eq!(replay["state"], state);
        assert_eq!(replay["request_id"], "0");
    }
    assert!(records.get("255").is_some());
    assert_eq!(records.values().count(), 256);
    assert!(!records.update("absent", json!({"ok":true})));
    let mut records = Records::default();
    let msg = message("0123456789abcdef", Some("retry"), browse());
    assert_eq!(
        records.dispatch(&msg, &mut |_, _| error("busy", "user editing"))["error"]["code"],
        "busy"
    );
    assert!(records.get("retry").is_none());
    assert_eq!(
        records.dispatch(&msg, &mut |_, _| json!({"ok":true,"state":"complete"}))["state"],
        "complete"
    );
    for id in [None, Some(""), Some("x".repeat(129).as_str())] {
        assert_eq!(
            records.dispatch(
                &message("0123456789abcdef", id, browse()),
                &mut |_, _| panic!("invalid ID")
            )["error"]["code"],
            "invalid_request"
        );
    }
}
#[test]
fn ui_commands_are_unsupported_but_modifications_are_recorded() {
    let mut server = Server::start_in(&directory()).unwrap();
    for (n, operation) in [
        Operation::Inspect,
        browse(),
        Operation::Open {
            relative_to: "self".into(),
            place: Place::Right,
            content: Content::Shell { cwd: None },
            focus: false,
        },
        Operation::Close {
            target: CloseTarget::Pane(1),
            confirmation: None,
            confirm_shells: false,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let id = n.to_string();
        let result = round_trip(&mut server, operation, Some(&id));
        assert_eq!(result["error"]["code"], "unsupported");
        if n != 0 {
            assert_eq!(
                round_trip(&mut server, Operation::Request { request: id }, None),
                result
            );
        }
    }
    assert_eq!(
        round_trip(
            &mut server,
            Operation::Request {
                request: "missing".into()
            },
            None
        )["error"]["code"],
        "request_unavailable"
    );
}

#[test]
fn raw_non_web_browse_never_reaches_ui_and_parameter_change_conflicts() {
    let mut records = Records::default();
    for url in [
        "javascript:80",
        "file:///tmp/fixture",
        "ftp://example.com",
        "localhost:3000",
    ] {
        let message = message(
            "0123456789abcdef",
            Some("bad"),
            Operation::Browse {
                url: url.into(),
                focus: false,
            },
        );
        assert_eq!(
            records.dispatch(&message, &mut |_, _| panic!("non-web URL reached UI"))["error"]["code"],
            "invalid_request"
        );
    }
    let original = message("0123456789abcdef", Some("same"), browse());
    records.dispatch(&original, &mut |_, _| json!({"ok":true,"state":"accepted"}));
    let mut changed = original;
    changed.operation = Operation::Browse {
        url: "https://example.com".into(),
        focus: true,
    };
    assert_eq!(
        records.dispatch(&changed, &mut |_, _| panic!("changed parameters replayed"))["error"]["code"],
        "request_conflict"
    );
}

#[test]
fn runtime_below_non_sticky_writable_ancestor_is_refused() {
    let ancestor = directory();
    fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o777)).unwrap();
    let private = ancestor.join("private");
    fs::DirBuilder::new().mode(0o700).create(&private).unwrap();
    assert!(
        Server::start_in(&private).is_err(),
        "writable ancestor must not be trusted"
    );
    fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o1777)).unwrap();
    let server = Server::start_in(&private).unwrap();
    assert_eq!(
        exchange_in(&private, &message(&server.id, None, Operation::Instances)).unwrap()["ok"],
        true
    );
    fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o775)).unwrap();
    assert!(exchange_in(&private, &message(&server.id, None, Operation::Instances)).is_err());
}

#[test]
fn client_checks_kernel_peer_uid_before_sending_any_message() {
    let dir = directory();
    let path = dir.join("peer.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let rejected = connect_as(&path, unsafe { libc::geteuid() }.wrapping_add(1));
    assert!(rejected.unwrap_err().to_string().contains("peer UID"));
    let (mut accepted, _) = listener.accept().unwrap();
    let mut buffer = [0; 1];
    assert_eq!(
        accepted.read(&mut buffer).unwrap(),
        0,
        "identity must not be sent to a rejected peer"
    );
    // Also verify the real same-UID kernel credential path, without changing process UID.
    let _client = connect(&path).unwrap();
    let (stream, _) = listener.accept().unwrap();
    check_peer(&stream, unsafe { libc::geteuid() }).unwrap();
}
#[test]
fn server_rejects_kernel_peer_uid_before_enqueueing() {
    let dir = directory();
    let server =
        Server::start_checked(&dir, unsafe { libc::geteuid() }.wrapping_add(1), || {}).unwrap();
    let mut stream = UnixStream::connect(&server.path).unwrap();
    assert_eq!(
        read_json(&mut stream).unwrap()["error"]["code"],
        "peer_uid_mismatch"
    );
    let mut buffer = [0; 1];
    assert_eq!(stream.read(&mut buffer).unwrap(), 0);
    assert!(server.incoming.try_recv().is_err());
}
#[test]
fn directory_swap_between_last_check_and_bind_is_rejected() {
    let root = directory();
    let original = root.join("run");
    fs::DirBuilder::new().mode(0o700).create(&original).unwrap();
    let replacement = root.join("foreign");
    fs::DirBuilder::new()
        .mode(0o777)
        .create(&replacement)
        .unwrap();
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o777)).unwrap();
    let result = Server::start_checked(&original, unsafe { libc::geteuid() }, || {
        fs::rename(&original, root.join("saved")).unwrap();
        std::os::unix::fs::symlink(&replacement, &original).unwrap();
    });
    assert!(result.is_err());
    assert_eq!(fs::metadata(&replacement).unwrap().mode() & 0o7777, 0o777);
    // A failed bind race must not leave a running listener in the replacement.
    for entry in fs::read_dir(&replacement).unwrap() {
        assert!(UnixStream::connect(entry.unwrap().path()).is_err());
    }
}
#[test]
fn runtime_cleanup_stays_in_the_opened_directory_after_ancestor_swap() {
    let root = directory();
    let original = root.join("run");
    let server = Server::start_in(&original).unwrap();
    let name = server.path.file_name().unwrap().to_owned();
    fs::rename(&original, root.join("saved")).unwrap();
    fs::create_dir(&original).unwrap();
    fs::write(original.join(&name), "foreign file").unwrap();
    drop(server);
    assert_eq!(
        fs::read_to_string(original.join(&name)).unwrap(),
        "foreign file"
    );
    assert!(!root.join("saved").join(name).exists());
}

#[test]
fn discovery_crosses_unrelated_and_stale_entries_before_a_live_instance() {
    let dir = directory();
    let mut entries = Vec::new();
    for n in 0..1024 {
        let path = dir.join(format!("{n:016x}.sock"));
        drop(UnixListener::bind(&path).unwrap());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        entries.push(Ok(path));
    }
    let unrelated = dir.join("unrelated.txt");
    fs::write(&unrelated, "keep me").unwrap();
    entries.insert(0, Ok(unrelated.clone()));
    let server = Server::start_in(&dir).unwrap();
    entries.push(Ok(server.path.clone()));
    // Supply a deterministic filesystem order with the live entry last.
    let found = instances_from(&dir, &Caller::default(), entries).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["instance"], server.id);
    assert_eq!(fs::read_to_string(unrelated).unwrap(), "keep me");
}

#[test]
fn client_refuses_directory_replacement_after_socket_metadata_check() {
    let root = directory();
    let original = root.join("run");
    let server = Server::start_in(&original).unwrap();
    let mut replacement = None;
    let result = exchange_checked(
        &original,
        &message(&server.id, None, Operation::Instances),
        || {
            fs::rename(&original, root.join("saved")).unwrap();
            fs::DirBuilder::new().mode(0o700).create(&original).unwrap();
            replacement =
                Some(UnixListener::bind(original.join(format!("{}.sock", server.id))).unwrap());
        },
    );
    assert!(result.unwrap_err().to_string().contains("identity changed"));
    let (mut stream, _) = replacement.unwrap().accept().unwrap();
    let mut byte = [0];
    assert_eq!(
        stream.read(&mut byte).unwrap(),
        0,
        "a swapped endpoint must not receive the request"
    );
}
#[test]
fn a_browser_cache_folder_named_paddock_does_not_stop_ctl() {
    // The Browser's WebKit makes `paddock` under the same base, readable by others.
    let base = directory();
    fs::DirBuilder::new()
        .mode(0o755)
        .create(base.join("paddock"))
        .unwrap();
    fs::set_permissions(base.join("paddock"), fs::Permissions::from_mode(0o755)).unwrap();
    let dir = runtime_dir_in(&base).unwrap();
    assert_eq!(dir.file_name().unwrap(), DIR);
    assert_ne!(DIR, "paddock");
    let server = Server::start_in(&dir).unwrap();
    assert!(
        server
            .path
            .starts_with(base.canonicalize().unwrap().join(DIR))
    );
    // WebKit's folder is left as it was.
    assert_eq!(
        fs::metadata(base.join("paddock")).unwrap().mode() & 0o7777,
        0o755
    );
}

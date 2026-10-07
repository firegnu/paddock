// From Saddle `src/control_cli.rs` at commit `f7d1bbafc84102edfef49b47bf49d5df90e2b2e3`.
// Adapted for paddock: private temporary runtime, transport-owned records, Browser and GPUI handoff.
use super::*;
use crate::control::Place;
use std::collections::BTreeMap;

const HELP: &str = "paddock ctl (JSON output)\n\
  instances\n\
  inspect [--instance ID]\n\
  open [--instance ID] [--relative-to self|active|PANE] --place tab|left|right|up|down\n\
       (--shell [--cwd PATH] | --agent NAME | --name NAME [--cwd PATH] [--role ROLE] [--prompt TEXT] -- PROGRAM ARG...)\n\
       [--focus] [--request-id ID]\n\
  browse URL [--focus] [--instance ID] [--request-id ID]\n\
  request REQUEST --instance ID\n\
  close (--pane PANE|--tab TAB) --instance ID [--request-id ID]\n\
        [--confirmation TOKEN --confirm-shells]\n\n\
Self uses CORRAL_NAME + CORRAL_INSTANCE, or shell PADDOCK_INSTANCE/PANE.\n\
Poll request with the returned instance and request_id. Retry uncertain operations with the SAME request ID and arguments.\n\
Runtime: $XDG_RUNTIME_DIR/paddock, otherwise $TMPDIR/paddock (private base required).";

pub fn run(args: Vec<String>) -> i32 {
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        println!("{}", json!({"ok":true,"help":HELP}));
        return 0;
    }
    let result = execute(args).unwrap_or_else(|e| error("invalid_request", format!("{e:#}")));
    println!("{result}");
    if result["ok"] == true { 0 } else { 1 }
}
fn parse(args: Vec<String>, caller: Caller) -> Result<Message> {
    let verb = args.first().context("missing ctl command")?;
    let mut values = BTreeMap::new();
    let mut command = Vec::new();
    let mut request = None;
    let mut url = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--" && verb != "open" {
            bail!("unexpected command separator");
        }
        if arg == "--" {
            command.extend(iter.cloned());
            break;
        }
        if verb == "request" && !arg.starts_with('-') && request.is_none() {
            request = Some(arg.clone());
            continue;
        }
        if verb == "browse" && !arg.starts_with('-') && url.is_none() {
            url = Some(arg.clone());
            continue;
        }
        let allowed = match verb.as_str() {
            "browse" => &["--instance", "--request-id", "--focus"][..],
            "inspect" => &["--instance"][..],
            "open" => &[
                "--instance",
                "--relative-to",
                "--place",
                "--shell",
                "--cwd",
                "--agent",
                "--name",
                "--role",
                "--prompt",
                "--focus",
                "--request-id",
            ][..],
            "close" => &[
                "--instance",
                "--pane",
                "--tab",
                "--request-id",
                "--confirmation",
                "--confirm-shells",
            ][..],
            "request" => &["--instance"][..],
            "instances" => &[][..],
            _ => bail!("unknown ctl command; use paddock ctl --help"),
        };
        if !allowed.contains(&arg.as_str()) {
            bail!("unknown option {arg}");
        }
        let value = if matches!(arg.as_str(), "--shell" | "--focus" | "--confirm-shells") {
            "true".into()
        } else {
            iter.next().context("option needs a value")?.clone()
        };
        if values.insert(arg.as_str(), value).is_some() {
            bail!("duplicate option {arg}");
        }
    }
    let instance = values.remove("--instance");
    let request_id = values.remove("--request-id");
    if let Some(id) = &instance {
        socket_path(std::path::Path::new("/"), id)?;
    }
    if let Some(id) = &request_id {
        anyhow::ensure!(
            !id.is_empty() && id.len() <= 128,
            "request_id must contain 1–128 bytes"
        );
    }
    let operation = match verb.as_str() {
        "browse" => Operation::Browse {
            url: browse_url(&url.context("browse needs a URL")?)?,
            focus: values.remove("--focus").is_some(),
        },
        "instances" => Operation::Instances,
        "inspect" => Operation::Inspect,
        "request" => {
            if instance.is_none() {
                bail!("request requires --instance");
            }
            Operation::Request {
                request: request.context("request needs an ID")?,
            }
        }
        "open" => {
            let place = match values.remove("--place").as_deref() {
                Some("tab") => Place::Tab,
                Some("left") => Place::Left,
                Some("right") => Place::Right,
                Some("up") => Place::Up,
                Some("down") => Place::Down,
                _ => bail!("--place must be tab, left, right, up or down"),
            };
            let relative_to = values
                .remove("--relative-to")
                .unwrap_or_else(|| "self".into());
            anyhow::ensure!(
                matches!(relative_to.as_str(), "self" | "active")
                    || relative_to.parse::<u64>().is_ok(),
                "relative-to must be self, active or a pane ID"
            );
            let focus = values.remove("--focus").is_some();
            let shell = values.remove("--shell").is_some();
            let agent = values.remove("--agent");
            let name = values.remove("--name");
            if usize::from(shell) + usize::from(agent.is_some()) + usize::from(name.is_some()) != 1
            {
                bail!("choose exactly one of --shell, --agent or --name");
            }
            let cwd = values
                .remove("--cwd")
                .map(|p| -> Result<String> {
                    let path = expand_home(&p)?.canonicalize().context("invalid cwd")?;
                    if !path.is_dir() {
                        bail!("cwd must be a directory");
                    }
                    Ok(path.to_string_lossy().into_owned())
                })
                .transpose()?;
            let content = if shell {
                Content::Shell { cwd }
            } else if let Some(name) = agent {
                if cwd.is_some() {
                    bail!("--agent does not accept --cwd");
                }
                Content::Agent { name }
            } else {
                let argv = std::mem::take(&mut command);
                if argv.first().is_none_or(|a| a.is_empty()) {
                    bail!("new agent needs -- PROGRAM ARG...");
                }
                Content::NewAgent {
                    name: name.unwrap(),
                    cwd,
                    role: values.remove("--role").unwrap_or_else(|| "regular".into()),
                    prompt: values.remove("--prompt"),
                    argv,
                }
            };
            Operation::Open {
                relative_to,
                place,
                content,
                focus,
            }
        }
        "close" => {
            if instance.is_none() {
                bail!("close requires --instance");
            }
            let pane = values.remove("--pane");
            let tab = values.remove("--tab");
            let target = match (pane, tab) {
                (Some(id), None) => CloseTarget::Pane(id.parse().context("invalid pane ID")?),
                (None, Some(id)) => CloseTarget::Tab(id.parse().context("invalid tab ID")?),
                _ => bail!("close requires exactly one of --pane or --tab"),
            };
            Operation::Close {
                target,
                confirmation: values.remove("--confirmation"),
                confirm_shells: values.remove("--confirm-shells").is_some(),
            }
        }
        _ => bail!("unknown ctl command; use paddock ctl --help"),
    };
    if !values.is_empty() || !command.is_empty() {
        bail!("options do not apply to the chosen content/command");
    }
    let mut message = Message {
        instance: instance.unwrap_or_default(),
        caller,
        operation,
        request_id,
    };
    if message.operation.is_mutation() && message.request_id.is_none() {
        message.request_id = Some(random_id()?);
    }
    Ok(message)
}
fn execute(args: Vec<String>) -> Result<Value> {
    let mut message = parse(args, Caller::environment())?;
    let caller = &message.caller;
    if matches!(message.operation, Operation::Instances) {
        return Ok(json!({"ok":true,"instances":instances(caller)?}));
    }
    let mut instance = if message.instance.is_empty() {
        None
    } else {
        Some(message.instance.clone())
    };
    if instance.is_none() {
        // Corral identity always takes precedence over an inherited shell hint.
        if caller.name.is_none()
            && caller.corral_instance.is_none()
            && caller.paddock_instance.is_some()
        {
            instance = caller.paddock_instance.clone();
        } else {
            let mut found = instances(caller)?;
            if found.len() > 1 && (caller.name.is_some() || caller.corral_instance.is_some()) {
                for item in &mut found {
                    let inspect = Message {
                        instance: item["instance"].as_str().unwrap().into(),
                        request_id: None,
                        caller: caller.clone(),
                        operation: Operation::Inspect,
                    };
                    if let Ok(value) = exchange(&inspect)
                        && value["ok"] == true
                    {
                        *item = value;
                    }
                }
            }
            let own: Vec<_> = found
                .iter()
                .filter(|v| v["caller"]["pane"].is_u64())
                .collect();
            let selected = if own.len() == 1 {
                own[0]
            } else if own.is_empty() && found.len() == 1 {
                &found[0]
            } else {
                return Ok(error(
                    "ambiguous_instance",
                    "select --instance from paddock ctl instances",
                ));
            };
            instance = selected["instance"].as_str().map(str::to_owned);
        }
    }
    message.instance = instance.context("no paddock instance available")?;
    match exchange(&message) {
        Ok(mut value) => {
            value["instance"] = json!(message.instance);
            if message.request_id.is_some() {
                value["request_id"] = json!(message.request_id);
            }
            Ok(value)
        }
        Err(e) => {
            let mut value = error("instance_unavailable", e);
            value["instance"] = json!(message.instance);
            value["request_id"] = json!(message.request_id);
            value["state"] = json!("uncertain");
            Ok(value)
        }
    }
}

fn expand_home(path: &str) -> Result<PathBuf> {
    if path == "~" || path.starts_with("~/") {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        Ok(PathBuf::from(home).join(path.strip_prefix("~/").unwrap_or("")))
    } else {
        Ok(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn browse_accepts_web_and_local_addresses_only() {
        let parsed = parse(
            args(&[
                "browse",
                "localhost:3000",
                "--focus",
                "--request-id",
                "given",
            ]),
            Caller::default(),
        )
        .unwrap();
        assert_eq!(parsed.request_id.as_deref(), Some("given"));
        assert_eq!(
            parsed.operation,
            Operation::Browse {
                url: "http://localhost:3000".into(),
                focus: true
            }
        );
        for url in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "data:text/plain,x",
            "ftp://example.com",
            "http://",
            "https:///x",
        ] {
            assert!(
                parse(args(&["browse", url]), Caller::default()).is_err(),
                "{url}"
            );
        }
    }
    #[test]
    fn all_open_contents_places_and_close_options_parse_without_side_effects() {
        for place in ["tab", "left", "right", "up", "down"] {
            let msg = parse(
                args(&[
                    "open",
                    "--place",
                    place,
                    "--shell",
                    "--cwd",
                    "/",
                    "--relative-to",
                    "active",
                ]),
                Caller::default(),
            )
            .unwrap();
            assert!(msg.request_id.is_some());
            assert!(matches!(
                msg.operation,
                Operation::Open {
                    content: Content::Shell { .. },
                    ..
                }
            ));
        }
        let existing = parse(
            args(&["open", "--place", "right", "--agent", "group/existing"]),
            Caller::default(),
        )
        .unwrap();
        assert!(matches!(
            existing.operation,
            Operation::Open {
                content: Content::Agent { .. },
                focus: false,
                ..
            }
        ));
        let new = parse(
            args(&[
                "open",
                "--place",
                "left",
                "--name",
                "group/new",
                "--role",
                "reviewer",
                "--prompt",
                "read this",
                "--focus",
                "--",
                "codex",
                "--model",
                "example",
            ]),
            Caller::default(),
        )
        .unwrap();
        assert!(
            matches!(new.operation, Operation::Open {content:Content::NewAgent {role, prompt, argv, ..}, focus:true, ..}
            if role == "reviewer" && prompt.as_deref() == Some("read this") && argv == ["codex","--model","example"])
        );
        let close = parse(
            args(&[
                "close",
                "--instance",
                "0123456789abcdef",
                "--tab",
                "12",
                "--confirmation",
                "token",
                "--confirm-shells",
            ]),
            Caller::default(),
        )
        .unwrap();
        assert!(matches!(
            close.operation,
            Operation::Close {
                target: CloseTarget::Tab(12),
                confirmation: Some(_),
                confirm_shells: true
            }
        ));
        for input in [
            vec!["open", "--place", "left", "--shell", "--agent", "a"],
            vec![
                "open", "--place", "left", "--shell", "--name", "a", "--", "cmd",
            ],
            vec![
                "open", "--place", "left", "--agent", "a", "--name", "b", "--", "cmd",
            ],
            vec!["open", "--place", "left"],
            vec!["open", "--place", "current", "--shell"],
            vec!["open", "--place", "left", "--name", "a"],
            vec!["open", "--place", "left", "--shell", "--prompt", "bad"],
            vec!["open", "--place", "left", "--shell", "--role", "bad"],
            vec!["open", "--place", "left", "--agent", "a", "--cwd", "/"],
            vec!["open", "--place", "left", "--shell", "--", "cmd"],
            vec![
                "open",
                "--place",
                "left",
                "--shell",
                "--relative-to",
                "wrong",
            ],
            vec!["open", "--place", "left", "--shell", "--shell"],
            vec!["request", "id"],
            vec!["close", "--pane", "1"],
            vec![
                "close",
                "--instance",
                "0123456789abcdef",
                "--pane",
                "1",
                "--tab",
                "2",
            ],
            vec!["inspect", "--"],
            vec!["inspect", "--focus"],
            vec!["inspect", "--instance", ""],
            vec!["browse", "https://example.com", "extra"],
            vec!["browse"],
        ] {
            assert!(parse(args(&input), Caller::default()).is_err(), "{input:?}");
        }
    }
}

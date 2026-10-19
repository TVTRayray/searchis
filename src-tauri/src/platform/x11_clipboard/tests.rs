//! 在私有 Xvfb 上验证选区持有者；没有安装 Xvfb 时跳过。不触碰真实桌面的剪贴板。

use std::{
    io::Read,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{
            AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, Property, WindowClass,
        },
        Event,
    },
    rust_connection::RustConnection,
    COPY_DEPTH_FROM_PARENT, CURRENT_TIME, NONE,
};

use super::*;
use x11rb::wrapper::ConnectionExt as _;

struct Xvfb {
    child: Child,
    display: String,
}

impl Xvfb {
    fn start() -> Option<Self> {
        let mut child = Command::new("Xvfb")
            .args(["-displayfd", "1", "-nolisten", "tcp", "-ac", "-screen", "0", "64x64x24"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut stdout = child.stdout.take()?;
        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        while stdout.read_exact(&mut byte).is_ok() && byte[0] != b'\n' {
            line.push(byte[0]);
        }
        let number = String::from_utf8(line).ok()?;
        Some(Self {
            child,
            display: format!(":{}", number.trim()),
        })
    }
}

impl Drop for Xvfb {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Requestor {
    conn: RustConnection,
    window: u32,
}

impl Requestor {
    fn new(display: &str) -> Self {
        let (conn, screen) = RustConnection::connect(Some(display)).unwrap();
        let root = conn.setup().roots[screen].root;
        let window = conn.generate_id().unwrap();
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .unwrap();
        conn.flush().unwrap();
        Self { conn, window }
    }

    fn atom(&self, name: &str) -> Atom {
        self.conn.intern_atom(false, name.as_bytes()).unwrap().reply().unwrap().atom
    }

    fn next_event(&self) -> Event {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(event) = self.conn.poll_for_event().unwrap() {
                return event;
            }
            assert!(Instant::now() < deadline, "timed out waiting for an X event");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// 取 CLIPBOARD 的某个 target；被拒绝时返回 `None`。支持 INCR。
    fn fetch(&self, target: &str) -> Option<(Atom, Vec<u8>)> {
        let clipboard = self.atom("CLIPBOARD");
        let incr = self.atom("INCR");
        let property = self.atom("_TEST_DEST");
        let target = self.atom(target);
        self.conn
            .convert_selection(self.window, clipboard, target, property, CURRENT_TIME)
            .unwrap();
        self.conn.flush().unwrap();
        loop {
            if let Event::SelectionNotify(notify) = self.next_event() {
                if notify.property == NONE {
                    return None;
                }
                break;
            }
        }
        let read = |delete| {
            self.conn
                .get_property(delete, self.window, property, AtomEnum::ANY, 0, u32::MAX / 4)
                .unwrap()
                .reply()
                .unwrap()
        };
        let first = read(true);
        if first.type_ != incr {
            return Some((first.type_, first.value));
        }
        let mut data = Vec::new();
        loop {
            if let Event::PropertyNotify(notify) = self.next_event() {
                if notify.window == self.window
                    && notify.atom == property
                    && notify.state == Property::NEW_VALUE
                {
                    let chunk = read(true);
                    if chunk.value.is_empty() {
                        return Some((chunk.type_, data));
                    }
                    data.extend_from_slice(&chunk.value);
                }
            }
        }
    }

    fn targets(&self) -> Vec<String> {
        let (_, bytes) = self.fetch("TARGETS").unwrap();
        bytes
            .chunks_exact(4)
            .map(|c| {
                let atom = u32::from_ne_bytes(c.try_into().unwrap());
                let name = self.conn.get_atom_name(atom).unwrap().reply().unwrap().name;
                String::from_utf8(name).unwrap()
            })
            .collect()
    }
}

fn with_server(test: impl FnOnce(&str)) {
    match Xvfb::start() {
        Some(server) => test(&server.display),
        None => eprintln!("skipped: Xvfb not available"),
    }
}

fn formats() -> Vec<(String, Vec<u8>)> {
    vec![
        ("text/html".into(), b"<b>hi</b>".to_vec()),
        ("text/plain".into(), "你好".as_bytes().to_vec()),
        ("UTF8_STRING".into(), "你好".as_bytes().to_vec()),
        ("XML Spreadsheet".into(), b"<Workbook/>".to_vec()),
    ]
}

#[test]
fn serves_targets_timestamp_and_each_format() {
    with_server(|display| {
        let clipboard = X11Clipboard::with_display(display);
        clipboard.set_formats(formats()).unwrap();
        let requestor = Requestor::new(display);

        let targets = requestor.targets();
        for expected in ["TARGETS", "TIMESTAMP", "text/html", "text/plain", "UTF8_STRING", "XML Spreadsheet"] {
            assert!(targets.iter().any(|t| t == expected), "{expected} missing in {targets:?}");
        }
        let (kind, stamp) = requestor.fetch("TIMESTAMP").unwrap();
        assert_eq!(kind, requestor.atom("INTEGER"));
        assert_eq!(stamp.len(), 4);
        for (name, bytes) in formats() {
            let (kind, got) = requestor.fetch(&name).unwrap();
            assert_eq!(got, bytes, "{name}");
            let expected_type = if name == "UTF8_STRING" { "UTF8_STRING" } else { &name };
            assert_eq!(kind, requestor.atom(expected_type), "{name}");
        }
        assert!(requestor.fetch("image/png").is_none(), "unknown target must be refused");
    });
}

#[test]
fn large_data_is_sent_with_incr_and_arrives_intact() {
    with_server(|display| {
        let clipboard = X11Clipboard::with_display(display);
        let big: Vec<u8> = (0..1_500_000u32).map(|i| (i % 251) as u8).collect();
        clipboard
            .set_formats(vec![("XML Spreadsheet".into(), big.clone()), ("text/plain".into(), b"x".to_vec())])
            .unwrap();
        let requestor = Requestor::new(display);
        let (_, got) = requestor.fetch("XML Spreadsheet").unwrap();
        assert_eq!(got.len(), big.len());
        assert!(got == big, "INCR payload differs");
        // 传输结束后同一请求方还能正常取小数据。
        assert_eq!(requestor.fetch("text/plain").unwrap().1, b"x");
    });
}

#[test]
fn set_formats_replaces_previous_data_atomically() {
    with_server(|display| {
        let clipboard = X11Clipboard::with_display(display);
        clipboard.set_formats(formats()).unwrap();
        clipboard
            .set_formats(vec![("text/uri-list".into(), b"file:///a\r\n".to_vec())])
            .unwrap();
        let requestor = Requestor::new(display);
        assert!(requestor.fetch("text/html").is_none());
        assert_eq!(requestor.fetch("text/uri-list").unwrap().1, b"file:///a\r\n");
    });
}

#[test]
fn selection_clear_releases_data_after_another_program_takes_over() {
    with_server(|display| {
        let clipboard = X11Clipboard::with_display(display);
        clipboard.set_formats(formats()).unwrap();
        let other = Requestor::new(display);
        other
            .conn
            .set_selection_owner(other.window, other.atom("CLIPBOARD"), CURRENT_TIME)
            .unwrap();
        other.conn.flush().unwrap();
        let owner = {
            let slot = lock(&clipboard.owner);
            Arc::clone(slot.as_ref().unwrap())
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while !lock(&owner.state).formats.is_empty() {
            assert!(Instant::now() < deadline, "SelectionClear never released the data");
            std::thread::sleep(Duration::from_millis(5));
        }
        // 我们已不是所有者，再次写入应重新夺回。
        clipboard.set_formats(formats()).unwrap();
        assert_eq!(Requestor::new(display).fetch("text/html").unwrap().1, b"<b>hi</b>");
    });
}

#[test]
fn unreachable_display_reports_format_copy_failed() {
    let clipboard = X11Clipboard::with_display(":9999");
    let error = clipboard.set_formats(formats()).unwrap_err();
    assert_eq!(error.code, "FORMAT_COPY_FAILED");
}

#[test]
fn incr_chunks_cover_all_bytes_then_one_empty_chunk() {
    let mut transfer = Incr {
        requestor: 1,
        property: 2,
        target: 3,
        data: Arc::new((0..10u8).collect()),
        offset: 0,
        finished: false,
        started: Instant::now(),
    };
    let mut collected = Vec::new();
    let mut sizes = Vec::new();
    while let Some(chunk) = transfer.next_chunk(4) {
        sizes.push(chunk.len());
        collected.extend_from_slice(chunk);
    }
    assert_eq!(sizes, [4, 4, 2, 0]);
    assert_eq!(collected, (0..10u8).collect::<Vec<_>>());
    assert!(transfer.next_chunk(4).is_none());
}

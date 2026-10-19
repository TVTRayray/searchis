//! X11 CLIPBOARD 选区持有者（SPEC-23 带格式复制）。
//!
//! Tauri 剪贴板插件不支持自定义 MIME 格式，所以带格式复制由 Searchis 自己持有选区：
//! 独立线程拥有一个不可见窗口，响应 `TARGETS`、`TIMESTAMP` 与各数据格式；
//! 数据超过单次属性上限时走 INCR；收到 `SelectionClear` 后释放数据。
//! Searchis 退出后富文本丢失属于已接受的限制（E-26）。
//!
//! 日志不记录剪贴板内容与文件名。

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use x11rb::{
    connection::RequestConnection as _,
    wrapper::ConnectionExt as _,
    connection::Connection,
    protocol::{
        xproto::{
            Atom, AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, CreateWindowAux,
            EventMask, PropMode, Property, SelectionNotifyEvent, WindowClass, SELECTION_NOTIFY_EVENT,
        },
        Event,
    },
    rust_connection::RustConnection,
    COPY_DEPTH_FROM_PARENT, CURRENT_TIME, NONE,
};

use crate::error::AppError;

/// 带格式复制使用的剪贴板写入口。生产实现持有 X11 选区；E2E 构建注入记录替身。
pub trait FormatClipboard: Send + Sync {
    /// 原子替换剪贴板内容为 `formats`（格式名 → 字节）。确认成为所有者后才返回成功。
    fn set_formats(&self, formats: Vec<(String, Vec<u8>)>) -> Result<(), AppError>;
}

fn failed(message: &str) -> AppError {
    AppError::new("FORMAT_COPY_FAILED", message)
}

/// 单次 `ChangeProperty` 的数据上限；更大的数据走 INCR。
const MAX_CHUNK: usize = 256 * 1024;
/// 一次 INCR 传输的存活上限，超过则丢弃（请求方已消失）。
const INCR_TTL: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------- INCR 状态机（纯逻辑）

/// 一次进行中的 INCR 传输。
#[derive(Debug)]
struct Incr {
    requestor: u32,
    property: Atom,
    target: Atom,
    data: Arc<Vec<u8>>,
    offset: usize,
    finished: bool,
    started: Instant,
}

impl Incr {
    /// 下一块数据；全部发完后返回一次空块（通知请求方结束），之后返回 `None`。
    fn next_chunk(&mut self, chunk: usize) -> Option<&[u8]> {
        if self.finished {
            return None;
        }
        if self.offset >= self.data.len() {
            self.finished = true;
            return Some(&[]);
        }
        let end = (self.offset + chunk).min(self.data.len());
        let slice = &self.data[self.offset..end];
        self.offset = end;
        Some(slice)
    }
}

// ---------------------------------------------------------------- 共享状态

struct Format {
    atom: Atom,
    data: Arc<Vec<u8>>,
}

#[derive(Default)]
struct State {
    formats: Vec<Format>,
    acquired: u32,
    transfers: Vec<Incr>,
}

struct Atoms {
    clipboard: Atom,
    targets: Atom,
    timestamp: Atom,
    incr: Atom,
    utf8_string: Atom,
    integer: Atom,
    ts_probe: Atom,
}

struct Owner {
    conn: RustConnection,
    window: u32,
    atoms: Atoms,
    state: Mutex<State>,
    /// 取得选区时间戳的握手：`set_formats` 写入探针属性，事件线程收到 PropertyNotify 后放入时间。
    probe: (Mutex<Option<u32>>, Condvar),
    alive: std::sync::atomic::AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Owner {
    fn connect(display: Option<&str>) -> Result<Arc<Self>, AppError> {
        let (conn, screen_num) = RustConnection::connect(display)
            .map_err(|_| failed("无法连接 X11 显示。带格式复制需要 X11 会话。"))?;
        let screen = conn.setup().roots[screen_num].clone();
        let intern = |name: &str| -> Result<Atom, AppError> {
            conn.intern_atom(false, name.as_bytes())
                .map_err(|_| failed("X11 通信失败。"))?
                .reply()
                .map(|reply| reply.atom)
                .map_err(|_| failed("X11 通信失败。"))
        };
        let atoms = Atoms {
            clipboard: intern("CLIPBOARD")?,
            targets: intern("TARGETS")?,
            timestamp: intern("TIMESTAMP")?,
            incr: intern("INCR")?,
            utf8_string: intern("UTF8_STRING")?,
            integer: intern("INTEGER")?,
            ts_probe: intern("_SEARCHIS_TS_PROBE")?,
        };
        let window = conn.generate_id().map_err(|_| failed("X11 通信失败。"))?;
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            screen.root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .map_err(|_| failed("无法创建 X11 窗口。"))?;
        conn.flush().map_err(|_| failed("X11 通信失败。"))?;
        let owner = Arc::new(Self {
            conn,
            window,
            atoms,
            state: Mutex::new(State::default()),
            probe: (Mutex::new(None), Condvar::new()),
            alive: std::sync::atomic::AtomicBool::new(true),
        });
        let worker = Arc::clone(&owner);
        std::thread::Builder::new()
            .name("x11-clipboard".into())
            .spawn(move || worker.event_loop())
            .map_err(|_| failed("无法启动剪贴板线程。"))?;
        Ok(owner)
    }

    fn is_alive(&self) -> bool {
        self.alive.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn chunk_size(&self) -> usize {
        MAX_CHUNK.min(self.conn.maximum_request_bytes().saturating_sub(1024)).max(1024)
    }

    // ------------------------------------------------------------ 事件线程

    fn event_loop(&self) {
        loop {
            match self.conn.wait_for_event() {
                Ok(event) => {
                    if self.handle(event).is_err() {
                        // 单个请求方失败（窗口已销毁等）不影响后续请求。
                        let _ = self.conn.flush();
                    }
                }
                Err(_) => break,
            }
        }
        self.alive.store(false, std::sync::atomic::Ordering::SeqCst);
        lock(&self.state).formats.clear();
        // 唤醒可能在等待时间戳的调用方。
        self.probe.1.notify_all();
    }

    fn handle(&self, event: Event) -> Result<(), x11rb::errors::ReplyOrIdError> {
        match event {
            Event::SelectionRequest(request) => self.on_request(&request),
            Event::SelectionClear(clear) => {
                if clear.owner == self.window && clear.selection == self.atoms.clipboard {
                    let mut state = lock(&self.state);
                    state.formats.clear();
                    state.transfers.clear();
                }
                Ok(())
            }
            Event::PropertyNotify(notify) => {
                if notify.window == self.window
                    && notify.atom == self.atoms.ts_probe
                    && notify.state == Property::NEW_VALUE
                {
                    *lock(&self.probe.0) = Some(notify.time);
                    self.probe.1.notify_all();
                } else if notify.state == Property::DELETE {
                    self.on_incr_delete(notify.window, notify.atom)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn on_request(
        &self,
        request: &x11rb::protocol::xproto::SelectionRequestEvent,
    ) -> Result<(), x11rb::errors::ReplyOrIdError> {
        let property = if request.property == NONE {
            request.target
        } else {
            request.property
        };
        let served = self.serve(request, property)?;
        let notify = SelectionNotifyEvent {
            response_type: SELECTION_NOTIFY_EVENT,
            sequence: 0,
            time: request.time,
            requestor: request.requestor,
            selection: request.selection,
            target: request.target,
            property: if served { property } else { NONE },
        };
        self.conn
            .send_event(false, request.requestor, EventMask::NO_EVENT, notify)?;
        self.conn.flush()?;
        Ok(())
    }

    /// 把请求的数据写入请求方窗口的属性。返回 false 表示拒绝。
    fn serve(
        &self,
        request: &x11rb::protocol::xproto::SelectionRequestEvent,
        property: Atom,
    ) -> Result<bool, x11rb::errors::ReplyOrIdError> {
        if request.selection != self.atoms.clipboard {
            return Ok(false);
        }
        let (formats, acquired): (Vec<(Atom, Arc<Vec<u8>>)>, u32) = {
            let state = lock(&self.state);
            (
                state
                    .formats
                    .iter()
                    .map(|f| (f.atom, Arc::clone(&f.data)))
                    .collect(),
                state.acquired,
            )
        };
        if formats.is_empty() {
            return Ok(false);
        }
        if request.time != CURRENT_TIME && acquired != 0 && request.time.wrapping_sub(acquired) > u32::MAX / 2 {
            return Ok(false); // 请求发生在我们取得选区之前
        }
        let requestor = request.requestor;
        if request.target == self.atoms.targets {
            let mut list = vec![self.atoms.targets, self.atoms.timestamp];
            list.extend(formats.iter().map(|(atom, _)| *atom));
            self.conn.change_property32(
                PropMode::REPLACE,
                requestor,
                property,
                AtomEnum::ATOM,
                &list,
            )?;
            return Ok(true);
        }
        if request.target == self.atoms.timestamp {
            self.conn.change_property32(
                PropMode::REPLACE,
                requestor,
                property,
                self.atoms.integer,
                &[acquired],
            )?;
            return Ok(true);
        }
        let Some((_, data)) = formats.into_iter().find(|(atom, _)| *atom == request.target) else {
            return Ok(false);
        };
        let kind = if request.target == self.atoms.utf8_string {
            self.atoms.utf8_string
        } else {
            request.target
        };
        let chunk = self.chunk_size();
        if data.len() <= chunk {
            self.conn
                .change_property8(PropMode::REPLACE, requestor, property, kind, &data)?;
            return Ok(true);
        }
        // INCR：先通知总长度，再在请求方每次删除属性后送下一块。
        self.conn.change_window_attributes(
            requestor,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            requestor,
            property,
            self.atoms.incr,
            &[u32::try_from(data.len()).unwrap_or(u32::MAX)],
        )?;
        let mut state = lock(&self.state);
        state.transfers.retain(|t| {
            t.started.elapsed() < INCR_TTL && !(t.requestor == requestor && t.property == property)
        });
        state.transfers.push(Incr {
            requestor,
            property,
            target: kind,
            data,
            offset: 0,
            finished: false,
            started: Instant::now(),
        });
        Ok(true)
    }

    fn on_incr_delete(
        &self,
        window: u32,
        property: Atom,
    ) -> Result<(), x11rb::errors::ReplyOrIdError> {
        let chunk = self.chunk_size();
        let mut state = lock(&self.state);
        let Some(index) = state
            .transfers
            .iter()
            .position(|t| t.requestor == window && t.property == property)
        else {
            return Ok(());
        };
        let transfer = &mut state.transfers[index];
        let target = transfer.target;
        let result = match transfer.next_chunk(chunk) {
            Some(bytes) => self
                .conn
                .change_property8(PropMode::REPLACE, window, property, target, bytes)
                .map(|_| ()),
            None => Ok(()),
        };
        if state.transfers[index].finished {
            state.transfers.remove(index);
        }
        drop(state);
        result?;
        self.conn.flush()?;
        Ok(())
    }

    // ------------------------------------------------------------ 调用方

    fn acquire(&self, formats: Vec<(String, Vec<u8>)>) -> Result<(), AppError> {
        let mut prepared = Vec::with_capacity(formats.len());
        for (name, data) in formats {
            let atom = self
                .conn
                .intern_atom(false, name.as_bytes())
                .map_err(|_| failed("X11 通信失败。"))?
                .reply()
                .map_err(|_| failed("X11 通信失败。"))?
                .atom;
            prepared.push(Format {
                atom,
                data: Arc::new(data),
            });
        }
        {
            let mut state = lock(&self.state);
            state.formats = prepared;
            state.transfers.clear();
        }
        // 零长度 APPEND 会产生 PropertyNotify，借它拿到服务器时间戳。
        *lock(&self.probe.0) = None;
        self.conn
            .change_property8(
                PropMode::APPEND,
                self.window,
                self.atoms.ts_probe,
                AtomEnum::STRING,
                &[],
            )
            .map_err(|_| failed("X11 通信失败。"))?;
        self.conn.flush().map_err(|_| failed("X11 通信失败。"))?;
        let time = {
            let guard = lock(&self.probe.0);
            let (guard, _) = self
                .probe
                .1
                .wait_timeout_while(guard, Duration::from_millis(500), |slot| {
                    slot.is_none() && self.is_alive()
                })
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.unwrap_or(CURRENT_TIME)
        };
        if !self.is_alive() {
            return Err(failed("X11 连接已断开。请重试。"));
        }
        self.conn
            .set_selection_owner(self.window, self.atoms.clipboard, time)
            .map_err(|_| failed("无法获取剪贴板所有权。"))?;
        let owner = self
            .conn
            .get_selection_owner(self.atoms.clipboard)
            .map_err(|_| failed("X11 通信失败。"))?
            .reply()
            .map_err(|_| failed("X11 通信失败。"))?
            .owner;
        if owner != self.window {
            lock(&self.state).formats.clear();
            return Err(failed("无法获取剪贴板所有权。请稍后重试。"));
        }
        lock(&self.state).acquired = time;
        Ok(())
    }
}

/// 生产实现：懒创建所有者线程；连接断开后下次调用重建。
pub struct X11Clipboard {
    display: Option<String>,
    owner: Mutex<Option<Arc<Owner>>>,
}

impl X11Clipboard {
    pub fn new() -> Self {
        Self {
            display: None,
            owner: Mutex::new(None),
        }
    }

    #[cfg(test)]
    fn with_display(display: &str) -> Self {
        Self {
            display: Some(display.to_owned()),
            owner: Mutex::new(None),
        }
    }
}

impl Default for X11Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatClipboard for X11Clipboard {
    fn set_formats(&self, formats: Vec<(String, Vec<u8>)>) -> Result<(), AppError> {
        let owner = {
            let mut slot = lock(&self.owner);
            match slot.as_ref().filter(|owner| owner.is_alive()) {
                Some(owner) => Arc::clone(owner),
                None => {
                    let owner = Owner::connect(self.display.as_deref())?;
                    *slot = Some(Arc::clone(&owner));
                    owner
                }
            }
        };
        owner.acquire(formats)
    }
}

// ---------------------------------------------------------------- E2E 替身

/// E2E 构建：不写剪贴板，把“格式名 → 字节长度/哈希”追加到 `SEARCHIS_E2E_CLIPBOARD_LOG`。
#[cfg(feature = "e2e-webdriver")]
pub struct RecordingClipboard;

#[cfg(feature = "e2e-webdriver")]
impl FormatClipboard for RecordingClipboard {
    fn set_formats(&self, formats: Vec<(String, Vec<u8>)>) -> Result<(), AppError> {
        use sha2::{Digest, Sha256};
        use std::io::Write;
        if std::env::var_os("SEARCHIS_E2E_CLIPBOARD_FAIL").is_some_and(|v| v == "1") {
            return Err(failed("E2E 注入的剪贴板失败。"));
        }
        let Some(log) = std::env::var_os("SEARCHIS_E2E_CLIPBOARD_LOG") else {
            return Ok(());
        };
        let entries: Vec<serde_json::Value> = formats
            .iter()
            .map(|(name, bytes)| {
                serde_json::json!({
                    "name": name,
                    "len": bytes.len(),
                    "sha256": hex::encode(Sha256::digest(bytes)),
                })
            })
            .collect();
        let line = serde_json::json!({ "formats": entries });
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log) {
            let _ = writeln!(file, "{line}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

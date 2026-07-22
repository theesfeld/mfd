//! Byte-level transports: serial path and Bluetooth RFCOMM.
//!
//! Linux Bluetooth: RFCOMM SPP sockets + optional BlueZ (`bluetoothctl`) assist
//! for power-on, connect, and OBD-like device discovery.

use crate::obd::error::{Error, Result};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

/// Bidirectional adapter link (ELM/STN text protocol on top).
pub trait Transport: Send {
    fn name(&self) -> &str;
    fn open(&mut self) -> Result<()>;
    fn write_all(&mut self, data: &[u8]) -> Result<()>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn set_timeout(&mut self, timeout: Duration) -> Result<()>;
    fn close(&mut self);

    fn write_str(&mut self, s: &str) -> Result<()> {
        self.write_all(s.as_bytes())
    }

    /// Read until ELM `>` prompt or timeout.
    fn read_until_prompt(&mut self, timeout: Duration) -> Result<String> {
        let start = Instant::now();
        let mut out = Vec::with_capacity(256);
        let mut buf = [0u8; 64];
        while start.elapsed() < timeout {
            match self.read(&mut buf) {
                Ok(0) => std::thread::sleep(Duration::from_millis(5)),
                Ok(n) => {
                    out.extend_from_slice(&buf[..n]);
                    if out.contains(&b'>') {
                        break;
                    }
                }
                Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if !out.is_empty() && out.contains(&b'>') {
                        break;
                    }
                    if out.is_empty() && start.elapsed() > timeout {
                        return Err(Error::Timeout);
                    }
                }
                Err(Error::Io(e))
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::Interrupted =>
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(e),
            }
        }
        if out.is_empty() {
            return Err(Error::Timeout);
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }
}

// ─── Serial (USB / existing rfcomm device node) ──────────────────────────────

pub struct SerialTransport {
    path: String,
    baud: u32,
    timeout: Duration,
    port: Option<Box<dyn serialport::SerialPort>>,
}

impl SerialTransport {
    pub fn new(path: impl Into<String>, baud: u32, timeout: Duration) -> Self {
        Self {
            path: path.into(),
            baud,
            timeout,
            port: None,
        }
    }
}

impl Transport for SerialTransport {
    fn name(&self) -> &str {
        &self.path
    }

    fn open(&mut self) -> Result<()> {
        let port = serialport::new(&self.path, self.baud)
            .timeout(self.timeout)
            .open()
            .map_err(|e| Error::Serial(e.to_string()))?;
        self.port = Some(port);
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        let p = self.port.as_mut().ok_or(Error::NotOpen)?;
        p.write_all(data).map_err(Error::Io)?;
        p.flush().map_err(Error::Io)?;
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let p = self.port.as_mut().ok_or(Error::NotOpen)?;
        p.read(buf).map_err(Error::Io)
    }

    fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.timeout = timeout;
        if let Some(p) = self.port.as_mut() {
            p.set_timeout(timeout)
                .map_err(|e| Error::Serial(e.to_string()))?;
        }
        Ok(())
    }

    fn close(&mut self) {
        self.port = None;
    }
}

// ─── Bluetooth classic SPP via Linux RFCOMM socket ───────────────────────────

#[cfg(target_os = "linux")]
mod bt {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::process::Command;

    const AF_BLUETOOTH: libc::c_int = 31;
    const BTPROTO_RFCOMM: libc::c_int = 3;
    /// Connect wait when socket is non-blocking (adapter may be waking).
    const CONNECT_WAIT: Duration = Duration::from_secs(8);

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct BdAddr {
        b: [u8; 6],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SockaddrRc {
        rc_family: libc::sa_family_t,
        rc_bdaddr: BdAddr,
        rc_channel: u8,
    }

    pub fn normalize_mac(mac: &str) -> Option<String> {
        let parts: Vec<&str> = mac.split([':', '-']).collect();
        if parts.len() != 6 {
            return None;
        }
        let mut out = Vec::with_capacity(6);
        for p in parts {
            if p.len() != 2 {
                return None;
            }
            let _ = u8::from_str_radix(p, 16).ok()?;
            out.push(p.to_ascii_uppercase());
        }
        Some(out.join(":"))
    }

    fn parse_bdaddr(mac: &str) -> Result<BdAddr> {
        let norm = normalize_mac(mac)
            .ok_or_else(|| Error::Adapter(format!("invalid Bluetooth MAC: {mac}")))?;
        let mut parts = [0u8; 6];
        for (i, p) in norm.split(':').enumerate() {
            parts[i] = u8::from_str_radix(p, 16)
                .map_err(|e| Error::Adapter(format!("bad MAC byte: {e}")))?;
        }
        // BlueZ stores BDADDR little-endian (reversed).
        parts.reverse();
        Ok(BdAddr { b: parts })
    }

    /// Name looks like a classic OBD / ELM / STN adapter.
    pub fn name_looks_like_obd(name: &str) -> bool {
        let u = name.to_ascii_uppercase();
        const KEYS: &[&str] = &[
            "OBD", "ELM", "STN", "OBDLINK", "VLINKER", "VEEPEAK", "OBDII", "OBD2", "MX+",
            "SCANTOOL", "PLX", "BAFX", "KONNWEI", "CARISTA", "FIXD", "LELINK", "CGDI",
        ];
        KEYS.iter().any(|k| u.contains(k))
    }

    /// Power controller on (best effort).
    pub fn bluez_power_on() {
        let _ = Command::new("bluetoothctl").args(["power", "on"]).output();
    }

    /// Ask BlueZ to connect classic profile (helps some dongles before RFCOMM).
    pub fn bluez_connect(mac: &str) {
        let Some(mac) = normalize_mac(mac) else {
            return;
        };
        let _ = Command::new("bluetoothctl")
            .args(["connect", &mac])
            .output();
    }

    /// Trust device (best effort; reduces re-pair prompts).
    pub fn bluez_trust(mac: &str) {
        let Some(mac) = normalize_mac(mac) else {
            return;
        };
        let _ = Command::new("bluetoothctl").args(["trust", &mac]).output();
    }

    /// Paired/known devices from `bluetoothctl devices` as `(mac, name)`.
    pub fn list_bluez_devices() -> Vec<(String, String)> {
        let Ok(out) = Command::new("bluetoothctl").args(["devices"]).output() else {
            return Vec::new();
        };
        if !out.status.success() {
            return Vec::new();
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut list = Vec::new();
        for line in text.lines() {
            // "Device AA:BB:CC:DD:EE:FF Name here"
            let line = line.trim();
            let rest = line.strip_prefix("Device ").unwrap_or(line);
            let mut parts = rest.splitn(2, char::is_whitespace);
            let Some(mac_raw) = parts.next() else {
                continue;
            };
            let Some(mac) = normalize_mac(mac_raw) else {
                continue;
            };
            let name = parts.next().unwrap_or("").trim().to_string();
            list.push((mac, name));
        }
        list
    }

    /// MACs that look like OBD adapters (paired/known), preferred MAC first if given.
    pub fn discover_obd_macs(preferred: Option<&str>) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(p) = preferred.and_then(normalize_mac) {
            out.push(p);
        }
        for (mac, name) in list_bluez_devices() {
            if name_looks_like_obd(&name) && !out.iter().any(|m| m == &mac) {
                out.push(mac);
            }
        }
        // Also include any device whose MAC was preferred already handled.
        // If nothing OBD-like found, keep preferred only (retry that forever).
        out
    }

    /// Common SPP channels to try after the configured channel fails.
    pub fn rfcomm_channel_candidates(preferred: u8) -> Vec<u8> {
        let mut ch = vec![preferred];
        for c in [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
            if !ch.contains(&c) {
                ch.push(c);
            }
        }
        ch
    }

    pub struct BtSppTransport {
        label: String,
        mac: String,
        channel: u8,
        timeout: Duration,
        fd: Option<OwnedFd>,
    }

    impl BtSppTransport {
        pub fn new(mac: impl Into<String>, channel: u8, timeout: Duration) -> Result<Self> {
            let raw = mac.into();
            let mac = normalize_mac(&raw)
                .ok_or_else(|| Error::Adapter(format!("invalid Bluetooth MAC: {raw}")))?;
            Ok(Self {
                label: format!("bt://{mac}:{channel}"),
                mac,
                channel,
                timeout,
                fd: None,
            })
        }

        pub fn mac(&self) -> &str {
            &self.mac
        }

        pub fn channel(&self) -> u8 {
            self.channel
        }
    }

    fn set_socket_timeouts(fd: libc::c_int, timeout: Duration) {
        let tv = libc::timeval {
            tv_sec: timeout.as_secs() as libc::time_t,
            tv_usec: timeout.subsec_micros() as libc::suseconds_t,
        };
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                &tv as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            );
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_SNDTIMEO,
                &tv as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            );
        }
    }

    impl Transport for BtSppTransport {
        fn name(&self) -> &str {
            &self.label
        }

        fn open(&mut self) -> Result<()> {
            // BlueZ assist: ensure adapter is powered and classic link is up.
            bluez_power_on();
            bluez_trust(&self.mac);
            bluez_connect(&self.mac);
            std::thread::sleep(Duration::from_millis(300));

            let fd = unsafe { libc::socket(AF_BLUETOOTH, libc::SOCK_STREAM, BTPROTO_RFCOMM) };
            if fd < 0 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }
            // Non-blocking connect with deadline so a dead dongle does not hang forever.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags >= 0 {
                unsafe {
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
            }
            let owned = unsafe { OwnedFd::from_raw_fd(fd) };
            let addr = SockaddrRc {
                rc_family: AF_BLUETOOTH as libc::sa_family_t,
                rc_bdaddr: parse_bdaddr(&self.mac)?,
                rc_channel: self.channel,
            };
            let rc = unsafe {
                libc::connect(
                    owned.as_raw_fd(),
                    &addr as *const _ as *const libc::sockaddr,
                    std::mem::size_of::<SockaddrRc>() as libc::socklen_t,
                )
            };
            if rc != 0 {
                let err = std::io::Error::last_os_error();
                let einprogress = err.raw_os_error() == Some(libc::EINPROGRESS)
                    || err.kind() == std::io::ErrorKind::WouldBlock;
                if !einprogress {
                    return Err(Error::Adapter(format!(
                        "RFCOMM connect {}:{}: {err}",
                        self.mac, self.channel
                    )));
                }
                // Wait for writable (connected) or error.
                let mut pfd = libc::pollfd {
                    fd: owned.as_raw_fd(),
                    events: libc::POLLOUT,
                    revents: 0,
                };
                let ms = CONNECT_WAIT.as_millis().min(i32::MAX as u128) as libc::c_int;
                let pr = unsafe { libc::poll(&mut pfd, 1, ms) };
                if pr == 0 {
                    return Err(Error::Adapter(format!(
                        "RFCOMM connect {}:{}: timeout ({CONNECT_WAIT:?})",
                        self.mac, self.channel
                    )));
                }
                if pr < 0 {
                    return Err(Error::Io(std::io::Error::last_os_error()));
                }
                let mut so_err: libc::c_int = 0;
                let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
                let gr = unsafe {
                    libc::getsockopt(
                        owned.as_raw_fd(),
                        libc::SOL_SOCKET,
                        libc::SO_ERROR,
                        &mut so_err as *mut _ as *mut libc::c_void,
                        &mut len,
                    )
                };
                if gr != 0 {
                    return Err(Error::Io(std::io::Error::last_os_error()));
                }
                if so_err != 0 {
                    return Err(Error::Adapter(format!(
                        "RFCOMM connect {}:{}: {}",
                        self.mac,
                        self.channel,
                        std::io::Error::from_raw_os_error(so_err)
                    )));
                }
            }
            // Back to blocking + I/O timeouts for ELM text protocol.
            let flags = unsafe { libc::fcntl(owned.as_raw_fd(), libc::F_GETFL) };
            if flags >= 0 {
                unsafe {
                    libc::fcntl(owned.as_raw_fd(), libc::F_SETFL, flags & !libc::O_NONBLOCK);
                }
            }
            set_socket_timeouts(owned.as_raw_fd(), self.timeout);
            self.fd = Some(owned);
            self.label = format!("bt://{}:{}", self.mac, self.channel);
            Ok(())
        }

        fn write_all(&mut self, data: &[u8]) -> Result<()> {
            let fd = self.fd.as_ref().ok_or(Error::NotOpen)?.as_raw_fd();
            let mut off = 0;
            while off < data.len() {
                let n = unsafe {
                    libc::write(
                        fd,
                        data[off..].as_ptr() as *const libc::c_void,
                        data.len() - off,
                    )
                };
                if n < 0 {
                    return Err(Error::Io(std::io::Error::last_os_error()));
                }
                off += n as usize;
            }
            Ok(())
        }

        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            let fd = self.fd.as_ref().ok_or(Error::NotOpen)?.as_raw_fd();
            let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n < 0 {
                return Err(Error::Io(std::io::Error::last_os_error()));
            }
            Ok(n as usize)
        }

        fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
            self.timeout = timeout;
            if let Some(fd) = self.fd.as_ref() {
                let tv = libc::timeval {
                    tv_sec: timeout.as_secs() as libc::time_t,
                    tv_usec: timeout.subsec_micros() as libc::suseconds_t,
                };
                unsafe {
                    libc::setsockopt(
                        fd.as_raw_fd(),
                        libc::SOL_SOCKET,
                        libc::SO_RCVTIMEO,
                        &tv as *const _ as *const libc::c_void,
                        std::mem::size_of::<libc::timeval>() as libc::socklen_t,
                    );
                }
            }
            Ok(())
        }

        fn close(&mut self) {
            self.fd = None;
        }
    }
}

#[cfg(target_os = "linux")]
pub use bt::{
    bluez_connect, bluez_power_on, bluez_trust, discover_obd_macs, list_bluez_devices,
    name_looks_like_obd, normalize_mac, rfcomm_channel_candidates, BtSppTransport,
};

#[cfg(not(target_os = "linux"))]
pub fn normalize_mac(mac: &str) -> Option<String> {
    let parts: Vec<&str> = mac.split([':', '-']).collect();
    if parts.len() != 6 {
        return None;
    }
    Some(
        parts
            .iter()
            .map(|p| p.to_ascii_uppercase())
            .collect::<Vec<_>>()
            .join(":"),
    )
}

#[cfg(not(target_os = "linux"))]
pub fn discover_obd_macs(preferred: Option<&str>) -> Vec<String> {
    preferred.and_then(normalize_mac).into_iter().collect()
}

#[cfg(not(target_os = "linux"))]
pub fn rfcomm_channel_candidates(preferred: u8) -> Vec<u8> {
    vec![preferred]
}

#[cfg(not(target_os = "linux"))]
pub fn bluez_power_on() {}

#[cfg(not(target_os = "linux"))]
pub fn bluez_connect(_mac: &str) {}

#[cfg(not(target_os = "linux"))]
pub fn name_looks_like_obd(name: &str) -> bool {
    let u = name.to_ascii_uppercase();
    u.contains("OBD") || u.contains("ELM")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_normalize() {
        assert_eq!(
            normalize_mac("00:04:3e:96:b8:f1").as_deref(),
            Some("00:04:3E:96:B8:F1")
        );
        assert_eq!(
            normalize_mac("00-04-3E-96-B8-F1").as_deref(),
            Some("00:04:3E:96:B8:F1")
        );
        assert!(normalize_mac("bad").is_none());
    }

    #[test]
    fn obd_name_filter() {
        assert!(name_looks_like_obd("OBDLink MX+ 41832"));
        assert!(name_looks_like_obd("ELM327 v1.5"));
        assert!(!name_looks_like_obd("HHKB-Hybrid_1"));
    }
}

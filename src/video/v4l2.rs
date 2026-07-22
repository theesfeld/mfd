//! Linux **V4L2** greyscale capture (dependency-free, libc ioctl).
//!
//! Supports **GREY** and **YUYV** (luma only). MJPEG webcams need a host
//! converter (`ffmpeg`) into GREY/YUYV or `MFD_FLIR_PATH` stills.

#![cfg(target_os = "linux")]

use super::GreyFrame;
use libc::{c_int, c_void, ioctl};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

// videodev2.h — fourcc
const V4L2_PIX_FMT_GREY: u32 = u32::from_le_bytes(*b"GREY");
const V4L2_PIX_FMT_YUYV: u32 = u32::from_le_bytes(*b"YUYV");
const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
const V4L2_MEMORY_MMAP: u32 = 1;
const V4L2_FIELD_NONE: u32 = 1;

// _IOWR('V', …)  — Linux ioctl encoding
fn ioc(dir: u32, nr: u32, size: u32) -> u64 {
    // _IOC(dir, 'V', nr, size)
    const IOC_NRBITS: u32 = 8;
    const IOC_TYPEBITS: u32 = 8;
    const IOC_SIZEBITS: u32 = 14;
    let type_ = u32::from(b'V');
    ((dir as u64) << (IOC_NRBITS + IOC_TYPEBITS + IOC_SIZEBITS))
        | ((type_ as u64) << (IOC_NRBITS + IOC_SIZEBITS))
        | ((nr as u64) << IOC_SIZEBITS)
        | (size as u64)
}
const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;

fn vidioc_s_fmt() -> u64 {
    ioc(
        IOC_READ | IOC_WRITE,
        5,
        std::mem::size_of::<V4l2Format>() as u32,
    )
}
fn vidioc_reqbufs() -> u64 {
    ioc(
        IOC_READ | IOC_WRITE,
        8,
        std::mem::size_of::<V4l2RequestBuffers>() as u32,
    )
}
fn vidioc_querybuf() -> u64 {
    ioc(
        IOC_READ | IOC_WRITE,
        9,
        std::mem::size_of::<V4l2Buffer>() as u32,
    )
}
fn vidioc_qbuf() -> u64 {
    ioc(
        IOC_READ | IOC_WRITE,
        15,
        std::mem::size_of::<V4l2Buffer>() as u32,
    )
}
fn vidioc_dqbuf() -> u64 {
    ioc(
        IOC_READ | IOC_WRITE,
        17,
        std::mem::size_of::<V4l2Buffer>() as u32,
    )
}
fn vidioc_streamon() -> u64 {
    ioc(IOC_WRITE, 18, std::mem::size_of::<u32>() as u32)
}
fn vidioc_streamoff() -> u64 {
    ioc(IOC_WRITE, 19, std::mem::size_of::<u32>() as u32)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct V4l2PixFormat {
    width: u32,
    height: u32,
    pixelformat: u32,
    field: u32,
    bytesperline: u32,
    sizeimage: u32,
    colorspace: u32,
    priv_: u32,
    flags: u32,
    ycbcr_enc: u32,
    quantization: u32,
    xfer_func: u32,
}

#[repr(C)]
union V4l2FormatFmt {
    pix: V4l2PixFormat,
    raw: [u8; 200],
}

#[repr(C)]
struct V4l2Format {
    type_: u32,
    fmt: V4l2FormatFmt,
}

#[repr(C)]
struct V4l2RequestBuffers {
    count: u32,
    type_: u32,
    memory: u32,
    capabilities: u32,
    flags: u8,
    reserved: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct V4l2Timecode {
    type_: u32,
    flags: u32,
    frames: u8,
    seconds: u8,
    minutes: u8,
    hours: u8,
    userbits: [u8; 4],
}

#[repr(C)]
union V4l2BufferM {
    offset: u32,
    userptr: usize,
    planes: usize,
    fd: i32,
}

#[repr(C)]
struct V4l2Buffer {
    index: u32,
    type_: u32,
    bytesused: u32,
    flags: u32,
    field: u32,
    timestamp: libc::timeval,
    timecode: V4l2Timecode,
    sequence: u32,
    memory: u32,
    m: V4l2BufferM,
    length: u32,
    reserved2: u32,
    request_fd: i32,
}

/// Open camera, grab one greyscale frame (stop stream after).
pub fn capture_once(device: &Path, want_w: u32, want_h: u32) -> Option<GreyFrame> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(device)
        .ok()?;
    let fd = file.as_raw_fd();

    // Prefer GREY then YUYV
    for fmt_code in [V4L2_PIX_FMT_GREY, V4L2_PIX_FMT_YUYV] {
        if let Some(f) = try_capture(fd, fmt_code, want_w, want_h) {
            return Some(f);
        }
    }
    // Fallback: single read() if driver supports it
    try_read_path(device, want_w, want_h)
}

fn try_capture(fd: RawFd, pixfmt: u32, want_w: u32, want_h: u32) -> Option<GreyFrame> {
    unsafe {
        let mut format: V4l2Format = std::mem::zeroed();
        format.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        format.fmt.pix = V4l2PixFormat {
            width: want_w,
            height: want_h,
            pixelformat: pixfmt,
            field: V4L2_FIELD_NONE,
            bytesperline: 0,
            sizeimage: 0,
            colorspace: 0,
            priv_: 0,
            flags: 0,
            ycbcr_enc: 0,
            quantization: 0,
            xfer_func: 0,
        };
        if ioctl(
            fd,
            vidioc_s_fmt() as _,
            &mut format as *mut _ as *mut c_void,
        ) < 0
        {
            return None;
        }
        let w = format.fmt.pix.width;
        let h = format.fmt.pix.height;
        let pf = format.fmt.pix.pixelformat;
        if w == 0 || h == 0 {
            return None;
        }

        let mut req: V4l2RequestBuffers = std::mem::zeroed();
        req.count = 2;
        req.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        req.memory = V4L2_MEMORY_MMAP;
        if ioctl(fd, vidioc_reqbufs() as _, &mut req as *mut _ as *mut c_void) < 0 || req.count < 1
        {
            return None;
        }

        let mut maps: Vec<(*mut u8, usize, u32)> = Vec::new();
        for i in 0..req.count {
            let mut buf: V4l2Buffer = std::mem::zeroed();
            buf.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
            buf.memory = V4L2_MEMORY_MMAP;
            buf.index = i;
            if ioctl(
                fd,
                vidioc_querybuf() as _,
                &mut buf as *mut _ as *mut c_void,
            ) < 0
            {
                cleanup_maps(&maps);
                return None;
            }
            let len = buf.length as usize;
            let off = buf.m.offset;
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                off as i64,
            );
            if ptr == libc::MAP_FAILED {
                cleanup_maps(&maps);
                return None;
            }
            maps.push((ptr as *mut u8, len, i));
            if ioctl(fd, vidioc_qbuf() as _, &mut buf as *mut _ as *mut c_void) < 0 {
                cleanup_maps(&maps);
                return None;
            }
        }

        let mut typ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        if ioctl(
            fd,
            vidioc_streamon() as _,
            &mut typ as *mut _ as *mut c_void,
        ) < 0
        {
            cleanup_maps(&maps);
            return None;
        }

        // Wait for a frame (poll up to ~1s)
        let mut got: Option<GreyFrame> = None;
        for _ in 0..50 {
            let mut pfd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let pr = libc::poll(&mut pfd, 1, 50);
            if pr <= 0 {
                continue;
            }
            let mut buf: V4l2Buffer = std::mem::zeroed();
            buf.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
            buf.memory = V4L2_MEMORY_MMAP;
            if ioctl(fd, vidioc_dqbuf() as _, &mut buf as *mut _ as *mut c_void) < 0 {
                break;
            }
            let idx = buf.index as usize;
            if idx < maps.len() {
                let (ptr, _len, _) = maps[idx];
                let used = buf.bytesused as usize;
                let slice = std::slice::from_raw_parts(ptr, used.min(maps[idx].1));
                got = Some(raw_to_grey(slice, w, h, pf));
            }
            let _ = ioctl(fd, vidioc_qbuf() as _, &mut buf as *mut _ as *mut c_void);
            break;
        }

        let mut typ = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        let _ = ioctl(
            fd,
            vidioc_streamoff() as _,
            &mut typ as *mut _ as *mut c_void,
        );
        cleanup_maps(&maps);
        got
    }
}

fn cleanup_maps(maps: &[(*mut u8, usize, u32)]) {
    for &(ptr, len, _) in maps {
        unsafe {
            libc::munmap(ptr as *mut c_void, len);
        }
    }
}

fn raw_to_grey(data: &[u8], w: u32, h: u32, pixfmt: u32) -> GreyFrame {
    let n = (w * h) as usize;
    let mut pixels = vec![0u8; n];
    if pixfmt == V4L2_PIX_FMT_GREY {
        let copy = n.min(data.len());
        pixels[..copy].copy_from_slice(&data[..copy]);
    } else {
        // YUYV: Y0 U Y1 V — take Y samples
        let mut i = 0usize;
        let mut o = 0usize;
        while o < n && i + 1 < data.len() {
            pixels[o] = data[i];
            o += 1;
            i += 2;
        }
    }
    // Downscale if huge (keep demo present snappy)
    if w > 320 || h > 240 {
        return downscale(&GreyFrame { w, h, pixels }, 320, 240);
    }
    GreyFrame { w, h, pixels }
}

fn downscale(src: &GreyFrame, mw: u32, mh: u32) -> GreyFrame {
    let scale = (src.w as f32 / mw as f32)
        .max(src.h as f32 / mh as f32)
        .max(1.0);
    let nw = ((src.w as f32) / scale) as u32;
    let nh = ((src.h as f32) / scale) as u32;
    let nw = nw.max(1);
    let nh = nh.max(1);
    let mut pixels = vec![0u8; (nw * nh) as usize];
    for y in 0..nh {
        for x in 0..nw {
            let sx = x * src.w / nw;
            let sy = y * src.h / nh;
            pixels[(y * nw + x) as usize] = src.pixels[(sy * src.w + sx) as usize];
        }
    }
    GreyFrame {
        w: nw,
        h: nh,
        pixels,
    }
}

/// Naive read fallback (some drivers).
fn try_read_path(device: &Path, _w: u32, _h: u32) -> Option<GreyFrame> {
    let mut f = OpenOptions::new().read(true).open(device).ok()?;
    let mut buf = vec![0u8; 640 * 480 * 2];
    let n = f.read(&mut buf).ok()?;
    if n < 100 {
        return None;
    }
    // Assume 320x240 YUYV if size matches-ish
    let frame = raw_to_grey(&buf[..n], 320, 240, V4L2_PIX_FMT_YUYV);
    let _ = f.seek(SeekFrom::Start(0));
    Some(frame)
}

/// Camera session: re-open device each grab (simple, robust for demo rates).
pub struct V4l2Source {
    pub device: std::path::PathBuf,
    pub width: u32,
    pub height: u32,
    pub last: Option<GreyFrame>,
    fail_count: u32,
}

impl V4l2Source {
    pub fn open(device: impl Into<std::path::PathBuf>) -> Self {
        Self {
            device: device.into(),
            width: 320,
            height: 240,
            last: None,
            fail_count: 0,
        }
    }

    pub fn from_env() -> Option<Self> {
        let dev = std::env::var("MFD_CAMERA")
            .or_else(|_| std::env::var("MFD_V4L2"))
            .ok()
            .filter(|s| !s.is_empty())?;
        let p = std::path::PathBuf::from(&dev);
        if p.exists() {
            Some(Self::open(p))
        } else {
            None
        }
    }

    /// Auto-pick first /dev/videoN if MFD_CAMERA=auto
    pub fn auto_detect() -> Option<Self> {
        if let Ok(v) = std::env::var("MFD_CAMERA") {
            if v == "auto" || v == "1" {
                for i in 0..8 {
                    let p = std::path::PathBuf::from(format!("/dev/video{i}"));
                    if p.exists() {
                        // try one capture
                        if capture_once(&p, 160, 120).is_some() {
                            return Some(Self::open(p));
                        }
                    }
                }
            }
        }
        Self::from_env()
    }

    pub fn grab(&mut self) -> Option<&GreyFrame> {
        match capture_once(&self.device, self.width, self.height) {
            Some(f) => {
                self.fail_count = 0;
                self.last = Some(f);
            }
            None => {
                self.fail_count = self.fail_count.saturating_add(1);
            }
        }
        self.last.as_ref()
    }
}

#[allow(dead_code)]
fn _ioctl_check(_: c_int) {}

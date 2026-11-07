//! Raw HID (vendor usage page) access via IOHIDManager. Vendor-page interfaces
//! need no Input Monitoring permission.

use std::ffi::c_void;
use std::os::raw::c_char;

type Ref = *mut c_void;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOHIDManagerCreate(alloc: Ref, options: u32) -> Ref;
    fn IOHIDManagerSetDeviceMatching(m: Ref, matching: Ref);
    fn IOHIDManagerOpen(m: Ref, options: u32) -> i32;
    fn IOHIDManagerCopyDevices(m: Ref) -> Ref;
    fn IOHIDDeviceGetProperty(d: Ref, key: Ref) -> Ref;
    fn IOHIDDeviceRegisterInputReportCallback(d: Ref, buf: *mut u8, len: isize, cb: ReportCallback, ctx: *mut c_void);
    fn IOHIDDeviceScheduleWithRunLoop(d: Ref, rl: Ref, mode: Ref);
    fn IOHIDDeviceSetReport(d: Ref, ty: u32, id: isize, report: *const u8, len: isize) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFDictionaryCreateMutable(alloc: Ref, cap: isize, kcb: *const c_void, vcb: *const c_void) -> Ref;
    fn CFDictionarySetValue(d: Ref, k: Ref, v: Ref);
    fn CFNumberCreate(alloc: Ref, ty: isize, v: *const c_void) -> Ref;
    fn CFNumberGetValue(n: Ref, ty: isize, out: *mut c_void) -> bool;
    fn CFStringCreateWithCString(alloc: Ref, s: *const c_char, enc: u32) -> Ref;
    fn CFSetGetCount(s: Ref) -> isize;
    fn CFSetGetValues(s: Ref, out: *mut Ref);
    fn CFRunLoopGetCurrent() -> Ref;
    fn CFRunLoopRunInMode(mode: Ref, secs: f64, return_after_source: bool) -> i32;
    fn CFRelease(r: Ref);
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    static kCFRunLoopDefaultMode: Ref;
}

type ReportCallback = unsafe extern "C" fn(*mut c_void, i32, *mut c_void, u32, u32, *mut u8, isize);

const NUMBER_INT: isize = 9; // kCFNumberIntType
const UTF8: u32 = 0x0800_0100;
const REPORT_OUTPUT: u32 = 1;

struct Inbox {
    buf: [u8; 64],
    reply: Option<[u8; 32]>,
}

pub struct RawHid {
    dev: Ref,
    inbox: Box<Inbox>,
}

unsafe extern "C" fn on_report(ctx: *mut c_void, _r: i32, _s: *mut c_void, _t: u32, _id: u32, rep: *mut u8, len: isize) {
    let inbox = &mut *(ctx as *mut Inbox);
    let mut out = [0u8; 32];
    let n = (len.max(0) as usize).min(32);
    out[..n].copy_from_slice(std::slice::from_raw_parts(rep, n));
    inbox.reply = Some(out);
}

unsafe fn cfstr(s: &[u8]) -> Ref {
    CFStringCreateWithCString(std::ptr::null_mut(), s.as_ptr() as *const c_char, UTF8)
}

unsafe fn set_int(d: Ref, key: &[u8], v: i32) {
    let k = cfstr(key);
    let n = CFNumberCreate(std::ptr::null_mut(), NUMBER_INT, &v as *const i32 as *const c_void);
    CFDictionarySetValue(d, k, n);
    CFRelease(n);
    CFRelease(k);
}

/// Open the first HID interface with this vendor ID and usage; returns it and its product ID.
pub fn open_raw_hid(vid: u16, usage_page: u16, usage: u16, _prefer: &[u16]) -> Result<(RawHid, u16), String> {
    unsafe {
        let m = IOHIDManagerCreate(std::ptr::null_mut(), 0);
        let d = CFDictionaryCreateMutable(
            std::ptr::null_mut(),
            0,
            &kCFTypeDictionaryKeyCallBacks as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const c_void,
        );
        set_int(d, b"VendorID\0", vid as i32);
        set_int(d, b"PrimaryUsagePage\0", usage_page as i32);
        set_int(d, b"PrimaryUsage\0", usage as i32);
        IOHIDManagerSetDeviceMatching(m, d);
        CFRelease(d);
        let r = IOHIDManagerOpen(m, 0);
        if r != 0 {
            return Err(format!("cannot open HID manager (IOReturn 0x{r:08x})"));
        }
        let set = IOHIDManagerCopyDevices(m);
        if set.is_null() || CFSetGetCount(set) == 0 {
            return Err(format!("no keyboard found with vendor 0x{vid:04X} raw HID interface"));
        }
        let mut devs = vec![std::ptr::null_mut(); CFSetGetCount(set) as usize];
        CFSetGetValues(set, devs.as_mut_ptr());
        let dev = devs[0];
        let key = cfstr(b"ProductID\0");
        let mut pid: i32 = 0;
        let p = IOHIDDeviceGetProperty(dev, key);
        CFRelease(key);
        if !p.is_null() {
            CFNumberGetValue(p, NUMBER_INT, &mut pid as *mut i32 as *mut c_void);
        }
        let mut inbox = Box::new(Inbox { buf: [0; 64], reply: None });
        let ctx = &mut *inbox as *mut Inbox as *mut c_void;
        IOHIDDeviceRegisterInputReportCallback(dev, inbox.buf.as_mut_ptr(), 64, on_report, ctx);
        IOHIDDeviceScheduleWithRunLoop(dev, CFRunLoopGetCurrent(), kCFRunLoopDefaultMode);
        // The manager and set stay alive for the life of the process (one short CLI call).
        Ok((RawHid { dev, inbox }, pid as u16))
    }
}

impl RawHid {
    /// Send one 32-byte report and wait up to 1 s for the reply that echoes its command bytes.
    pub fn transfer(&mut self, out: &[u8; 32]) -> Result<[u8; 32], String> {
        self.inbox.reply = None;
        let r = unsafe { IOHIDDeviceSetReport(self.dev, REPORT_OUTPUT, 0, out.as_ptr(), 32) };
        if r != 0 {
            return Err(format!("HID write failed (IOReturn 0x{r:08x})"));
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.05, true) };
            if let Some(rep) = self.inbox.reply.take() {
                if super::is_reply(out, &rep) {
                    return Ok(rep);
                }
            }
        }
        Err("keyboard did not answer".into())
    }
}

//! Raw HID (vendor usage page) access via the Windows HID class driver.

use std::ptr::{null, null_mut};
use windows_sys::core::GUID;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows_sys::Win32::Devices::HumanInterfaceDevice::{
    HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetHidGuid, HidD_GetPreparsedData, HidP_GetCaps, HIDD_ATTRIBUTES,
    HIDP_CAPS, HIDP_STATUS_SUCCESS, PHIDP_PREPARSED_DATA,
};
use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows_sys::Win32::System::IO::{CancelIo, GetOverlappedResult, OVERLAPPED};

pub struct RawHid {
    h: HANDLE,
    event: HANDLE,
    out_len: usize,
    in_len: usize,
}

impl Drop for RawHid {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.h);
            CloseHandle(self.event);
        }
    }
}

/// Usage page, usage and report lengths (including the report-ID byte) of an open HID handle.
unsafe fn caps(h: HANDLE) -> Option<HIDP_CAPS> {
    let mut pp: PHIDP_PREPARSED_DATA = 0;
    if !HidD_GetPreparsedData(h, &mut pp) {
        return None;
    }
    let mut c: HIDP_CAPS = std::mem::zeroed();
    let ok = HidP_GetCaps(pp, &mut c) == HIDP_STATUS_SUCCESS;
    HidD_FreePreparsedData(pp);
    ok.then_some(c)
}

/// Open the first HID interface with this vendor ID and usage; returns it and its product ID.
pub fn open_raw_hid(vid: u16, usage_page: u16, usage: u16, _prefer: &[u16]) -> Result<(RawHid, u16), String> {
    unsafe {
        let mut guid: GUID = std::mem::zeroed();
        HidD_GetHidGuid(&mut guid);
        let set = SetupDiGetClassDevsW(&guid, null(), null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
        let mut found = None;
        let mut i = 0;
        loop {
            let mut ifd: SP_DEVICE_INTERFACE_DATA = std::mem::zeroed();
            ifd.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
            if SetupDiEnumDeviceInterfaces(set, null(), &guid, i, &mut ifd) == 0 {
                break;
            }
            i += 1;
            let mut need = 0u32;
            SetupDiGetDeviceInterfaceDetailW(set, &ifd, null_mut(), 0, &mut need, null_mut());
            // u64 storage keeps the detail struct suitably aligned.
            let mut buf = vec![0u64; (need as usize).div_ceil(8) + 1];
            let det = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*det).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(set, &ifd, det, need, null_mut(), null_mut()) == 0 {
                continue;
            }
            let path = std::ptr::addr_of!((*det).DevicePath) as *const u16;
            let h = CreateFileW(
                path,
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                null_mut(),
            );
            if h == INVALID_HANDLE_VALUE {
                continue;
            }
            let mut attr: HIDD_ATTRIBUTES = std::mem::zeroed();
            attr.Size = std::mem::size_of::<HIDD_ATTRIBUTES>() as u32;
            let hit = if HidD_GetAttributes(h, &mut attr) && attr.VendorID == vid {
                caps(h).filter(|c| c.UsagePage == usage_page && c.Usage == usage)
            } else {
                None
            };
            match hit {
                Some(c) => {
                    found = Some((h, attr.ProductID, c));
                    break;
                }
                None => {
                    CloseHandle(h);
                }
            }
        }
        SetupDiDestroyDeviceInfoList(set);
        let (h, pid, c) = found.ok_or_else(|| format!("no keyboard found with vendor 0x{vid:04X} raw HID interface"))?;
        let event = CreateEventW(null(), 1, 0, null());
        Ok((
            RawHid { h, event, out_len: c.OutputReportByteLength as usize, in_len: c.InputReportByteLength as usize },
            pid,
        ))
    }
}

impl RawHid {
    /// Wait for one overlapped operation; cancel it after `ms`.
    unsafe fn finish(&self, ov: &mut OVERLAPPED, started: i32, ms: u32) -> Option<usize> {
        let mut n = 0u32;
        if started == 0 && windows_sys::Win32::Foundation::GetLastError() != 997 {
            return None; // not ERROR_IO_PENDING
        }
        if WaitForSingleObject(self.event, ms) != WAIT_OBJECT_0 {
            CancelIo(self.h);
            GetOverlappedResult(self.h, ov, &mut n, 1);
            return None;
        }
        (GetOverlappedResult(self.h, ov, &mut n, 0) != 0).then_some(n as usize)
    }

    /// Send one 32-byte report and wait up to 1 s for the reply that echoes its command bytes.
    pub fn transfer(&mut self, out: &[u8; 32]) -> Result<[u8; 32], String> {
        unsafe {
            let mut pkt = vec![0u8; self.out_len.max(33)]; // report ID 0, then the report
            pkt[1..33].copy_from_slice(out);
            let mut ov: OVERLAPPED = std::mem::zeroed();
            ov.hEvent = self.event;
            let s = WriteFile(self.h, pkt.as_ptr(), pkt.len() as u32, null_mut(), &mut ov);
            self.finish(&mut ov, s, 1000).ok_or("HID write failed")?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            let mut buf = vec![0u8; self.in_len.max(33)];
            loop {
                let left = deadline.saturating_duration_since(std::time::Instant::now()).as_millis() as u32;
                if left == 0 {
                    return Err("keyboard did not answer".into());
                }
                let mut ov: OVERLAPPED = std::mem::zeroed();
                ov.hEvent = self.event;
                let s = ReadFile(self.h, buf.as_mut_ptr(), buf.len() as u32, null_mut(), &mut ov);
                let Some(n) = self.finish(&mut ov, s, left) else { continue };
                if n < 33 {
                    continue;
                }
                let mut rep = [0u8; 32];
                rep.copy_from_slice(&buf[1..33]);
                if super::is_reply(out, &rep) {
                    return Ok(rep);
                }
            }
        }
    }
}

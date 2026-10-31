//! Free space on the disk a folder is on.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// Bytes available to an ordinary user on the filesystem holding `path`.
pub fn available_bytes(path: &Path) -> Option<u64> {
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `c_path` is a valid NUL-terminated string and `stat` is a valid
    // out-pointer; statvfs only writes through it, and `stat` is read only if it
    // returned 0.
    let ok = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) } == 0;
    if !ok {
        return None;
    }
    // SAFETY: statvfs returned 0, so it initialised `stat`.
    let stat = unsafe { stat.assume_init() };
    // The field types differ between platforms (u32 on macOS, u64 on Linux).
    #[allow(clippy::unnecessary_cast)]
    Some(stat.f_bavail as u64 * stat.f_frsize as u64)
}

const GB: f64 = 1_000_000_000.0;
const TB: f64 = 1_000_000_000_000.0;

/// "42 GB available", in decimal units as disks are sold; terabytes from 1000 GB.
pub fn format_available(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= TB {
        format!("{:.1} TB available", b / TB)
    } else if b >= GB {
        format!("{:.0} GB available", b / GB)
    } else {
        format!("{:.0} MB available", b / 1_000_000.0)
    }
}

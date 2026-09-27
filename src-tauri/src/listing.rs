use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use crate::scan::{is_link, mtime};

/// One directory entry; links and reparse points never make it here.
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
    /// Storage identity for files that may be hard-linked elsewhere.
    pub id: Option<u128>,
}

/// Remembers which files were already counted, so a hard link adds its size only once.
pub struct Links {
    shards: Vec<Mutex<HashSet<u128>>>,
}

impl Default for Links {
    fn default() -> Self {
        Self {
            shards: (0..64).map(|_| Mutex::default()).collect(),
        }
    }
}

impl Links {
    /// True the first time an id is seen.
    pub fn first(&self, id: u128) -> bool {
        let shard = &self.shards[(id as usize ^ (id >> 64) as usize) % self.shards.len()];
        shard.lock().unwrap_or_else(|e| e.into_inner()).insert(id)
    }
}

pub fn list(dir: &Path, with_ids: bool) -> io::Result<Vec<Entry>> {
    #[cfg(windows)]
    if with_ids {
        if let Ok(v) = win::list(dir) {
            return Ok(v);
        }
    }
    list_std(dir, with_ids)
}

fn list_std(dir: &Path, with_ids: bool) -> io::Result<Vec<Entry>> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir)?.flatten() {
        let Ok(m) = e.metadata() else { continue };
        if is_link(&m) {
            continue;
        }
        out.push(Entry {
            name: e.file_name().to_string_lossy().into_owned(),
            is_dir: m.is_dir(),
            size: if m.is_dir() { 0 } else { m.len() },
            modified: mtime(&m),
            id: if with_ids { unix_id(&m) } else { None },
        });
    }
    Ok(out)
}

#[cfg(unix)]
fn unix_id(m: &fs::Metadata) -> Option<u128> {
    use std::os::unix::fs::MetadataExt;
    (m.is_file() && m.nlink() > 1).then(|| (u128::from(m.dev()) << 64) | u128::from(m.ino()))
}

#[cfg(not(unix))]
fn unix_id(_: &fs::Metadata) -> Option<u128> {
    None
}

/// Whether file ids from this volume are unique and stable enough to dedupe hard links.
pub fn ids_supported(root: &Path) -> bool {
    #[cfg(windows)]
    {
        win::is_ntfs(root)
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        true
    }
}

#[cfg(windows)]
mod win {
    use super::Entry;
    use std::ffi::OsStr;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::*;

    /// 100 ns ticks between 1601-01-01 and 1970-01-01.
    const EPOCH_DIFF: i64 = 116_444_736_000_000_000;

    fn wide(s: &OsStr) -> Vec<u16> {
        s.encode_wide().chain(Some(0)).collect()
    }

    /// Verbatim form so node_modules-deep paths are not cut at MAX_PATH.
    fn verbatim(p: &Path) -> Vec<u16> {
        let s = p.to_string_lossy().replace('/', "\\");
        let v = if s.starts_with(r"\\?\") || !p.is_absolute() {
            s
        } else if let Some(unc) = s.strip_prefix(r"\\") {
            format!(r"\\?\UNC\{unc}")
        } else {
            format!(r"\\?\{s}")
        };
        wide(OsStr::new(&v))
    }

    /// Bulk listing with file ids in one call per 64 KB of entries (no per-file open).
    pub fn list(dir: &Path) -> io::Result<Vec<Entry>> {
        let path = verbatim(dir);
        // SAFETY: path is NUL-terminated; the handle is closed on every return below.
        let h = unsafe {
            CreateFileW(
                path.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };
        if h == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut buf = vec![0u64; 8192];
        let mut out = Vec::new();
        let mut class = FileIdBothDirectoryRestartInfo;
        let result = loop {
            // SAFETY: buf is 8-byte aligned and its byte length is passed.
            let ok = unsafe {
                GetFileInformationByHandleEx(
                    h,
                    class,
                    buf.as_mut_ptr().cast(),
                    (buf.len() * 8) as u32,
                )
            };
            if ok == 0 {
                // SAFETY: plain thread-local error read.
                let err = unsafe { GetLastError() };
                break if err == ERROR_NO_MORE_FILES {
                    Ok(())
                } else {
                    Err(io::Error::from_raw_os_error(err as i32))
                };
            }
            class = FileIdBothDirectoryInfo;
            parse(&buf, &mut out);
        };
        // SAFETY: h is a valid handle opened above.
        unsafe { CloseHandle(h) };
        result.map(|()| out)
    }

    fn parse(buf: &[u64], out: &mut Vec<Entry>) {
        let base = buf.as_ptr().cast::<u8>();
        let mut offset = 0usize;
        loop {
            // SAFETY: the OS wrote a chain of records inside buf; offsets come from NextEntryOffset.
            let info = unsafe { &*base.add(offset).cast::<FILE_ID_BOTH_DIR_INFO>() };
            let name_ptr = std::ptr::addr_of!(info.FileName).cast::<u16>();
            // SAFETY: FileNameLength is the byte length of the name that follows the record.
            let name =
                unsafe { std::slice::from_raw_parts(name_ptr, info.FileNameLength as usize / 2) };
            let name = String::from_utf16_lossy(name);
            let attrs = info.FileAttributes;
            if name != "." && name != ".." && attrs & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
                let is_dir = attrs & FILE_ATTRIBUTE_DIRECTORY != 0;
                out.push(Entry {
                    name,
                    is_dir,
                    size: if is_dir {
                        0
                    } else {
                        info.EndOfFile.max(0) as u64
                    },
                    modified: ((info.LastWriteTime - EPOCH_DIFF).max(0) / 10_000_000) as u64,
                    id: (!is_dir).then_some(info.FileId as u64 as u128),
                });
            }
            if info.NextEntryOffset == 0 {
                break;
            }
            offset += info.NextEntryOffset as usize;
        }
    }

    pub fn is_ntfs(root: &Path) -> bool {
        let path = wide(root.as_os_str());
        let mut vol = [0u16; 261];
        let mut fs_name = [0u16; 32];
        // SAFETY: buffers and their lengths are passed together; unused outputs are null.
        let ok = unsafe {
            GetVolumePathNameW(path.as_ptr(), vol.as_mut_ptr(), vol.len() as u32) != 0
                && GetVolumeInformationW(
                    vol.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    fs_name.as_mut_ptr(),
                    fs_name.len() as u32,
                ) != 0
        };
        ok && String::from_utf16_lossy(&fs_name).trim_end_matches('\0') == "NTFS"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_listing_matches_std_even_past_max_path() {
        let tmp = tempfile::tempdir().unwrap();
        // node_modules trees routinely exceed the 260-char MAX_PATH.
        let mut deep = tmp.path().to_path_buf();
        while deep.as_os_str().len() < 300 {
            deep.push("pasta-bem-comprida-çã");
        }
        fs::create_dir_all(deep.join("sub")).unwrap();
        fs::write(deep.join("ärquivo.txt"), b"12345").unwrap();

        let key = |mut v: Vec<Entry>| {
            v.sort_by(|a, b| a.name.cmp(&b.name));
            v.into_iter()
                .map(|e| (e.name, e.is_dir, e.size, e.modified))
                .collect::<Vec<_>>()
        };
        let fast = list(&deep, ids_supported(tmp.path())).unwrap();
        assert!(fast.iter().all(|e| e.is_dir || e.id.is_some()) || !cfg!(windows));
        let got = key(fast);
        assert_eq!(got, key(list_std(&deep, false).unwrap()));
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn links_report_first_sighting_only() {
        let l = Links::default();
        assert!(l.first(7));
        assert!(!l.first(7));
        assert!(l.first(7 << 64));
    }
}

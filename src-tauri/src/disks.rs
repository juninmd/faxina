use std::path::Path;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub mount: String,
    pub total: u64,
    pub free: u64,
}

/// Mounted volumes worth scanning: one entry per mount point, zero-size pseudo filesystems dropped.
#[tauri::command]
pub fn list_disks() -> Vec<DiskInfo> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut out: Vec<DiskInfo> = disks
        .list()
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| DiskInfo {
            mount: d.mount_point().to_string_lossy().into_owned(),
            total: d.total_space(),
            free: d.available_space(),
        })
        .collect();
    out.sort_by(|a, b| a.mount.cmp(&b.mount));
    out.dedup_by(|a, b| a.mount == b.mount);
    out
}

#[tauri::command]
pub fn disk_info(path: String) -> Option<DiskInfo> {
    let target = Path::new(&path);
    list_disks()
        .into_iter()
        .filter(|d| target.starts_with(&d.mount))
        .max_by_key(|d| d.mount.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_at_least_the_volume_holding_temp() {
        let disks = list_disks();
        assert!(!disks.is_empty());
        let tmp = std::env::temp_dir();
        let info =
            disk_info(tmp.to_string_lossy().into_owned()).expect("temp dir lives on a listed disk");
        assert!(info.total >= info.free);
    }
}

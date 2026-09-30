use std::{path::PathBuf, str::FromStr};

pub fn trim_custom(string: String, trim_val: &str) -> String {
    let mut result = string;
    let res_pre = result.strip_prefix(trim_val);
    if let Some(pre) = res_pre {
        result = pre.to_string();
    }

    let res_suf = result.strip_suffix(trim_val);
    if let Some(suf) = res_suf {
        result = suf.to_string();
    }

    return result;
}

pub fn same_path(path1: &str, path2: &str) -> bool {
    let path1 = PathBuf::from_str(path1);
    let path2 = PathBuf::from_str(path2);
    if let Ok(path1) = path1
        && let Ok(path2) = path2
    {
        return path1 == path2;
    } else {
        return false;
    }
}

/// Converts a file path (absolute/relative) to a local file path (relative to the folder being indexed).
///
/// Parameters:
/// - path: the file path to convert.
///
/// Returns:
/// - the local file path as a string.
pub fn to_local_path(path: String, folder: String) -> Option<String> {
    let local_file_path = path.strip_prefix(folder.as_str());

    if let Some(local_file_path) = local_file_path {
        let mut local_file_path = trim_custom(local_file_path.to_string(), "/");

        local_file_path = trim_custom(local_file_path, "\\");

        return Some(local_file_path);
    } else {
        return None;
    }
}

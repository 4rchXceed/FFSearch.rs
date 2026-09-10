use std::{
    path::{MAIN_SEPARATOR, PathBuf},
    str::FromStr,
};

pub fn join_path(base: &str, path: &str) -> String {
    let sep = MAIN_SEPARATOR;
    let mut base = base.to_string();
    if !base.ends_with(sep) {
        base.push(sep);
    }
    if path.starts_with(sep) {
        base.push_str(&path[1..]);
    } else {
        base.push_str(path);
    }
    return base;
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

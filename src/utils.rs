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
    if path1.is_err() || path2.is_err() {
        return false;
    }
    let path1 = path1.unwrap();
    let path2 = path2.unwrap();
    return path1 == path2;
}

pub fn trim_custom(string: String, trim_val: &str) -> String {
    let mut result = string;
    let res_pre = result.strip_prefix(trim_val);
    if res_pre.is_some() {
        result = res_pre.unwrap().to_string();
    }
    let res_suf = result.strip_suffix(trim_val);
    if res_suf.is_some() {
        result = res_suf.unwrap().to_string();
    }
    return result;
}

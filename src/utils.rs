use std::{
    path::{MAIN_SEPARATOR, PathBuf},
    str::FromStr,
};

#[macro_export]
macro_rules! not_null_check {
    ($name: ident, $call: expr, $if_null: expr) => {
        let $name = $call;
        if $name.is_none() {
            return $if_null;
        }
        #[allow(unused_variables)]
        let $name = $name.unwrap();
    };
}

#[macro_export]
macro_rules! not_err_check {
    ($name: ident, $call: expr, $if_null: expr) => {
        let $name = $call;
        if $name.is_err() {
            return $if_null;
        }
        #[allow(unused_variables)]
        let $name = $name.unwrap();
    };
}

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

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;

    #[test]
    fn test_join_path() {
        assert_eq!(join_path("C:\\", "test.txt"), "C:\\test.txt");
        assert_eq!(join_path("C:\\", "\\test.txt"), "C:\\test.txt");
        assert_eq!(join_path("\\root", "miku-v4x.zip"), "\\root\\miku-v4x.zip");
        assert_eq!(join_path("root", "\\miku-v4x.zip"), "root\\miku-v4x.zip");
    }

    #[test]
    fn test_same_path() {
        assert!(same_path("C:\\test.txt", "C:\\test.txt"));
        assert!(!same_path("C:\\test.txt", "C:\\test2.txt"));
        assert!(same_path("\\root\\miku-v4x.zip", "\\root\\miku-v4x.zip"));
        assert!(!same_path("\\root\\miku-v4x.zip", "root\\miku-v4x.zip"));
    }

    #[test]
    fn test_not_null_check() {
        assert!(!null_check_this_returns_false());
        assert!(null_check_this_returns_true());
    }

    #[test]
    fn test_err_check() {
        assert!(!err_check_this_returns_false());
        assert!(err_check_this_returns_true());
    }

    #[test]
    fn test_trim_custom() {
        assert_eq!(trim_custom("test".to_string(), "t"), "es");
        assert_eq!(trim_custom("test".to_string(), "te"), "st");
        assert_eq!(trim_custom("test".to_string(), "st"), "te");
    }

    fn null_check_this_returns_false() -> bool {
        let null: Option<()> = None;
        not_null_check!(null, null, false);
        return true;
    }
    fn null_check_this_returns_true() -> bool {
        let null: Option<()> = Some(());
        not_null_check!(null, null, false);
        return true;
    }

    fn err_check_this_returns_true() -> bool {
        let err: Result<(), ()> = Ok(());
        not_err_check!(err, err, false);
        return true;
    }
    fn err_check_this_returns_false() -> bool {
        let err: Result<(), ()> = Err(());
        not_err_check!(err, err, false);
        return true;
    }
}

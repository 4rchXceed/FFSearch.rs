use std::path::MAIN_SEPARATOR;

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

// https://github.com/olback/tray-item-rs/blob/master/examples/windows-edit-menu-items/build.rs

#[cfg(windows)]
use windres::Build;

#[cfg(windows)]
fn main() {
    Build::new().compile("tray-example.rc").unwrap();
}

#[cfg(not(windows))]
fn main() {
}

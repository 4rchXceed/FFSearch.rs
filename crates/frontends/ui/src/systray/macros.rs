#[macro_export]
macro_rules! tray_resource {
    ($name: expr) => {
        include_bytes!(concat!("../../resources/", stringify!($name)))
    };
}

#[cfg(not(windows))]
use crate::tray_resource;
#[cfg(not(windows))]
use std::io::Cursor;

use tray_item::IconSource;

use crate::systray::errors::IconCreationError;

#[cfg(not(windows))]
pub fn load_icon_linux() -> Result<IconSource, IconCreationError> {
    let cursor = Cursor::new(tray_resource!(icon.png));

    let decoder = image::codecs::png::PngDecoder::new(cursor)
        .map_err(|e| IconCreationError::CouldNotDecodeIcon(e.to_string()))?;

    let buffer = image::DynamicImage::from_decoder(decoder)
        .map_err(|e| IconCreationError::InvalidIcon(e.to_string()))?;

    let icon_source = IconSource::Data {
        height: buffer.height() as i32,
        width: buffer.width() as i32,
        data: buffer.to_rgba8().into_raw(),
    };

    return Ok(icon_source);
}

#[cfg(windows)]
pub fn load_icon_windows() -> Result<IconSource, IconCreationError> {
    let icon_source = IconSource::Resource("tray-icon");

    return Ok(icon_source);
}

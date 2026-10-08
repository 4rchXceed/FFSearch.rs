use std::io::Cursor;

use image::ImageReader;
use tray::Icon;

use crate::{systray::errors::IconCreationError, tray_resource};

pub fn load_icon() -> Result<Icon, IconCreationError> {
    let image = ImageReader::new(Cursor::new(tray_resource!(icon.png)))
        .with_guessed_format()
        .map_err(|e| IconCreationError::CouldNotDetectIconFormat(e.to_string()))?;

    // let dimensions = image
    //     .into_dimensions()
    //     .map_err(|e| IconCreationError::CouldNotReadIconDimensions(e.to_string()))?;

    let image_decoded = image
        .decode()
        .map_err(|e| IconCreationError::CouldNotDecodeIcon(e.to_string()))?;

    let rgba = image_decoded.to_rgba8();

    return Icon::from_rgba(
        rgba.as_raw().clone(),
        image_decoded.width(),
        image_decoded.height(),
    )
    .map_err(|e| IconCreationError::InvalidIcon(e.to_string()));
}

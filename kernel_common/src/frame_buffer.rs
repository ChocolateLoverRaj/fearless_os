use crate::config::{CONFIG, ScreenResolution};

pub trait GetScreenResolution {
    fn resolution(&self) -> ScreenResolution;
}

/// Get the index of the resolution we should choose.
/// First the preffered resolutions list is searched.
/// If no preffered resolution is found, the highest resolution is chosen.
pub fn choose_resolution<I: Iterator<Item = ScreenResolution>>(
    mut resolutions: impl FnMut() -> I,
) -> Option<usize> {
    // First check if our preffered mode is available
    if !CONFIG.preffered_resolutions.is_empty() {
        if let Some(index) = CONFIG
            .preffered_resolutions
            .iter()
            .find_map(|preffered_resolution| {
                resolutions().position(|resolution| &resolution == preffered_resolution)
            })
        {
            return Some(index);
        }
    }
    // Otherwise, find the highest resolution mode
    // Currently this logic just finds the mode with the most pixels * bits per pixel
    resolutions()
        .enumerate()
        .max_by_key(|(_, resolution)| {
            u64::from(resolution.width) * u64::from(resolution.height) * u64::from(resolution.bpp)
        })
        .map(|(index, _)| index)
}

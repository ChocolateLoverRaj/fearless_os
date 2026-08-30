use core::ptr::NonNull;

use common::{
    bios::{BiosFns, vesa::VesaModeAttributes},
    paging::LeafMappingFlags,
    pat::WRITE_COMBINING_INDEX,
};

use crate::{
    config::CONFIG, frame_buffer_embedded_graphics::FrameBufferEmbeddedGraphics, logger,
    memory::map_phys,
};

pub fn init(bios_fns: BiosFns) {
    let vbe_info = bios_fns.get_vbe_info().unwrap();
    // log::info!("VBE: {vbe_info:#X?}");
    let vbe_version = vbe_info.info().vbe_version;
    log::info!("VBE version: {vbe_version:#X?}.");
    if vbe_version != [0x00, 0x03] {
        panic!("Unsupported VESA version: {vbe_version:#X?}.",);
    }

    let mode_list = vbe_info.mode_list();
    let mode_to_use = {
        let mut mode_to_use = None;
        // First check if our preffered mode is available
        if let Some(preffered) = CONFIG.preffered_resolution {
            mode_to_use = mode_list.into_iter().find_map(|mode| {
                let info = bios_fns.vesa_get_mode_info(mode).unwrap();
                if info.x_resolution.get() == preffered.width
                    && info.y_resolution.get() == preffered.height
                    && info.bits_per_pixel == preffered.bpp
                {
                    Some((mode, info))
                } else {
                    None
                }
            });
        }
        // Otherwise, find the highest resolution mode
        if mode_to_use.is_none() {
            for video_mode in mode_list {
                let info = bios_fns.vesa_get_mode_info(video_mode).unwrap();
                log::trace!("Video mode: {video_mode:#X}: {info:#X?}");
                let attributes = VesaModeAttributes::new_with_raw_value(info.mode_attributs.get());
                if attributes.mode_type() && attributes.linear_frame_buffer_mode_available() {
                    let width = info.x_resolution.get();
                    let height = info.y_resolution.get();
                    let bpp = info.bits_per_pixel;
                    log::info!("Graphics mode: {width}x{height}, {bpp}-bit color");
                    if mode_to_use.is_none_or(|(_, best_info)| {
                        width >= best_info.x_resolution.get()
                            && height >= best_info.y_resolution.get()
                            && bpp >= best_info.bits_per_pixel
                    }) {
                        mode_to_use = Some((video_mode, info));
                    }
                }
            }
        }
        mode_to_use
    };

    if let Some((mode, info)) = mode_to_use {
        let width = info.x_resolution.get();
        let height = info.y_resolution.get();
        let bpp = info.bits_per_pixel;
        let ptr = info.phys_base_ptr.get();
        log::info!("Setting mode to: {mode}, {width}x{height}, {bpp}-bit color, {ptr:#X}");

        let width_bytes = info.lin_bytes_per_scan.get();
        let bytes_to_map = u32::from(width_bytes) * u32::from(height);
        let frame_buffer_virt_addr = map_phys(
            ptr.into(),
            bytes_to_map.into(),
            LeafMappingFlags {
                writable: true,
                executable: false,
                user_mode_accessible: false,
                pat_index: WRITE_COMBINING_INDEX,
            },
        )
        .unwrap();
        bios_fns.vesa_set_mode(mode, true).unwrap();
        let ptr = NonNull::new(frame_buffer_virt_addr as *mut u32).unwrap();
        let f = unsafe { FrameBufferEmbeddedGraphics::new(ptr, (&info).into()) };
        log::info!("Switching logger to frame buffer.");
        logger::init_frame_buffer(f, CONFIG.prefer_screen_logging);
        log::info!("Switched logger to frame buffer.");
    } else {
        log::info!("No suitable graphics mode found");
    }
}

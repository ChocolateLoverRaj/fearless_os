use core::ptr::NonNull;

use arbitrary_int::u9;
use common::{
    bios::{
        BiosFns,
        vesa::{ModeInfo, VesaModeAttributes},
    },
    paging::LeafMappingFlags,
    pat::WRITE_COMBINING_INDEX,
};

use crate::memory::map_phys;

pub fn init(bios_fns: BiosFns) {
    let vbe_info = bios_fns.get_vbe_info().unwrap();
    // log::info!("VBE: {vbe_info:#X?}");
    let vbe_version = vbe_info.info().vbe_version;
    log::info!("VBE version: {vbe_version:#X?}.");
    if vbe_version != [0x00, 0x03] {
        panic!("Unsupported VESA version: {vbe_version:#X?}.",);
    }

    let mut best_mode = None::<(u9, ModeInfo)>;
    for video_mode in vbe_info.mode_list() {
        let info = bios_fns.vesa_get_mode_info(video_mode).unwrap();
        log::debug!("Video mode: {video_mode:#X}: {info:#X?}");
        let attributes = VesaModeAttributes::new_with_raw_value(info.mode_attributs.get());
        if attributes.mode_type() && attributes.linear_frame_buffer_mode_available() {
            let width = info.x_resolution.get();
            let height = info.y_resolution.get();
            let bpp = info.bits_per_pixel;
            log::debug!("Graphics mode: {width}x{height}, {bpp}-bit color");
            if best_mode.is_none_or(|(_, best_info)| {
                width >= best_info.x_resolution.get()
                    && height >= best_info.y_resolution.get()
                    && bpp >= best_info.bits_per_pixel
            }) {
                best_mode = Some((video_mode, info));
            }
        }
    }
    if let Some((mode, info)) = best_mode {
        let width = info.x_resolution.get();
        let height = info.y_resolution.get();
        let bpp = info.bits_per_pixel;
        let ptr = info.phys_base_ptr.get();
        log::info!("Best mode: {mode}, {width}x{height}, {bpp}-bit color, {ptr:#X}");
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
        let mut frame_buffer_pointer = NonNull::slice_from_raw_parts(
            NonNull::new(frame_buffer_virt_addr as *mut u8).unwrap(),
            bytes_to_map.try_into().unwrap(),
        );
        let frame_buffer = unsafe { frame_buffer_pointer.as_mut() };
        frame_buffer.fill(255);
        loop {}
        // let ptr = usize::try_from(info.phys_base_ptr.get()).unwrap() as *mut u8;
        // let bytes = usize::try_from(info.lin_bytes_per_scan.get()).unwrap()
        //     * usize::try_from(height).unwrap()
        //     * usize::try_from(bpp).unwrap();
        // unsafe { ptr.write_bytes(0x67, bytes) };

        // log::info!("Set mode");
    } else {
        log::info!("No suitable graphics mode found");
    }
}

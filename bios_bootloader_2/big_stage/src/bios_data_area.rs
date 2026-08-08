use core::num::NonZero;

use zerocopy::little_endian::U16;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BiosDataArea {
    pub io_ports_com: [U16; 4],
    pub io_ports_lpt: [U16; 3],
    pub ebda_base_addr: U16,
    pub detected_hardware: U16,
    _padding_0: u8,
    pub kib_before_ebda_or_unusable_mem: U16,
    _padding_1: [u8; 2],
    pub keyboard_stage_flags: U16,
    _padding_2: [u8; 5],
    pub keyboard_buffer: [u8; 32],
    _padding_3: [u8; 11],
    pub display_mode: u8,
    pub number_of_columns_in_text_mode: u16,
    _padding_4: [u8; 23],
    pub video_base_io_port: U16,
    _padding_5: [u8; 6],
    pub n_irq_0_timer_ticks_since_boot: u16,
    _padding_6: [u8; 7],
    pub n_hdds_detected: u8,
    _padding_7: [u8; 9],
    pub keyboard_buffer_start: u16,
    pub keyboard_buffer_end: u16,
    _padding_8: [u8; 19],
    pub last_keyboard_led_or_shift_key_state: u8,
}

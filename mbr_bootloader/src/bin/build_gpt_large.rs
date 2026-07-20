use mbr_bootloader::build_test_image;

fn main() {
    build_test_image("gpt_large", 0x40000000000, 1, 1);
}

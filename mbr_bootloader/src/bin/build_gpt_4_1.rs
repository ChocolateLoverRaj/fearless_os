use mbr_bootloader::build_test_image;

fn main() {
    build_test_image("gpt_4_1", 512 * 68, 4, 1);
}

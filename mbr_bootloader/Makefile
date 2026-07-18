BUILD_DIR := build
COMMON_DEPS := Cargo.toml Cargo.lock src/lib.rs

.PHONY: default, test, clean

default: test

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/stage_0.bin: bootloader/stage_0.S src/bin/build_stage_0.rs $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run  --bin build_stage_0

$(BUILD_DIR)/stage_1.bin: bootloader/stage_1.S src/bin/build_stage_1.rs $(BUILD_DIR)/stage_0.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_stage_1

$(BUILD_DIR)/bootloader.bin: src/bin/build_combined.rs $(BUILD_DIR)/stage_0.bin $(BUILD_DIR)/stage_1.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_combined

$(BUILD_DIR)/test.bin: test.S src/bin/build_test.rs $(BUILD_DIR)/bootloader.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_test

$(BUILD_DIR)/gpt.img: src/bin/build_test_img.rs $(BUILD_DIR)/bootloader.bin $(BUILD_DIR)/test.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_test_img

$(BUILD_DIR)/gpt_large.img: src/bin/build_test_img_large.rs $(BUILD_DIR)/bootloader.bin $(BUILD_DIR)/test.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_test_img_large

test: $(BUILD_DIR)/gpt.img $(BUILD_DIR)/gpt_large.img
	cargo test

clean:
	rm -rf $(BUILD_DIR)

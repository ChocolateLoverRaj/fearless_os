BUILD_DIR := build
COMMON_DEPS := Cargo.toml Cargo.lock src/lib.rs

.PHONY: default, test, clean

default: test

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/bootloader.bin: bootloader.S src/bin/build_bootloader.rs $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_bootloader

$(BUILD_DIR)/test.bin: test.S src/bin/build_test.rs $(BUILD_DIR)/bootloader.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_test

$(BUILD_DIR)/gpt_1_1.img: src/bin/build_gpt_1_1.rs $(BUILD_DIR)/bootloader.bin $(BUILD_DIR)/test.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_gpt_1_1

$(BUILD_DIR)/gpt_1_128.img: src/bin/build_gpt_1_128.rs $(BUILD_DIR)/bootloader.bin $(BUILD_DIR)/test.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_gpt_1_128

$(BUILD_DIR)/gpt_4_1.img: src/bin/build_gpt_4_1.rs $(BUILD_DIR)/bootloader.bin $(BUILD_DIR)/test.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_gpt_4_1

$(BUILD_DIR)/gpt_large.img: src/bin/build_gpt_large.rs $(BUILD_DIR)/bootloader.bin $(BUILD_DIR)/test.bin $(COMMON_DEPS) | $(BUILD_DIR)
	cargo run --bin build_gpt_large

test: $(BUILD_DIR)/gpt_1_1.img $(BUILD_DIR)/gpt_1_128.img $(BUILD_DIR)/gpt_4_1.img $(BUILD_DIR)/gpt_large.img
	cargo test

clean:
	rm -rf $(BUILD_DIR)

BUILD_DIR := build

.PHONY: default, run, clean

default: run

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/stage_2.bin: stage_2.nasm | $(BUILD_DIR)
	nasm stage_2.nasm -f bin -o $(BUILD_DIR)/stage_2.bin

$(BUILD_DIR)/rust.bin: rust/Cargo.toml rust/Cargo.lock rust/src/main.rs | $(BUILD_DIR)
	cargo objcopy --bin bios_bootloader --target x86_64-unknown-none -- -O binary $(BUILD_DIR)/rust.bin

$(BUILD_DIR)/stage_0.bin: scripts/Cargo.toml scripts/Cargo.lock scripts/src/bin/stage_0.rs stage_0.nasm $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin | $(BUILD_DIR)
	cargo run --package scripts --bin stage_0

$(BUILD_DIR)/stage_1.bin: scripts/Cargo.toml scripts/Cargo.lock scripts/src/bin/stage_1.rs stage_1.nasm $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin $(BUILD_DIR)/stage_0.bin | $(BUILD_DIR)
	cargo run --package scripts --bin stage_1

$(BUILD_DIR)/disk.img: scripts/Cargo.toml scripts/Cargo.lock scripts/src/bin/disk.rs $(BUILD_DIR)/stage_0.bin $(BUILD_DIR)/stage_1.bin $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin | $(BUILD_DIR)
	cargo run --package scripts --bin disk


run: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
        --machine q35,accel=kvm:whpx:hvf:tcg \
        --no-reboot \
        --nographic \
        -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on

clean:
	rm -rf $(BUILD_DIR)

BUILD_DIR := build
R := scripts/Cargo.toml scripts/Cargo.lock scripts/src/lib.rs

.PHONY: default, run, run_with_graphic, clean

default: run

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/mbr_bootloader.bin: $(R) scripts/src/bin/mbr_bootloader.rs ../mbr_bootloader/bootloader.nasm | $(BUILD_DIR)
	cd scripts && cargo run --bin mbr_bootloader

$(BUILD_DIR)/stage_2.bin: $(R) scripts/src/bin/stage_2.rs stage_2.nasm | $(BUILD_DIR)
	cd scripts && cargo run --bin stage_2

.PHONY: $(BUILD_DIR)/rust.bin
$(BUILD_DIR)/rust.bin:
	cd rust && cargo objcopy --target x86_64-unknown-none -- -O binary ../$(BUILD_DIR)/rust.bin

.PHONY: $(BUILD_DIR)/rust
$(BUILD_DIR)/rust:
	cd rust && cargo build --target x86_64-unknown-none
	cp rust/target/x86_64-unknown-none/debug/bios_bootloader $(BUILD_DIR)/rust

$(BUILD_DIR)/stage_0.bin: $(R) scripts/src/bin/stage_0.rs stage_0.nasm $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin | $(BUILD_DIR)
	cd scripts && cargo run --bin stage_0

$(BUILD_DIR)/stage_1.bin: $(R) scripts/src/bin/stage_1.rs stage_1.nasm $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin $(BUILD_DIR)/rust $(BUILD_DIR)/stage_0.bin | $(BUILD_DIR)
	cd scripts && cargo run --bin stage_1

$(BUILD_DIR)/disk.img: $(R) scripts/src/bin/disk.rs $(BUILD_DIR)/mbr_bootloader.bin $(BUILD_DIR)/stage_0.bin $(BUILD_DIR)/stage_1.bin $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin | $(BUILD_DIR)
	cd scripts && cargo run --bin disk


run: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
        --machine q35,accel=kvm:whpx:hvf:tcg \
        --no-reboot \
        -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on \
        --nographic

run_with_graphic: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
        --machine q35,accel=kvm:whpx:hvf:tcg \
        --no-reboot \
        -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on

clean:
	rm -rf $(BUILD_DIR)

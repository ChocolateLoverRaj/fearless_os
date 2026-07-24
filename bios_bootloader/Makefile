BUILD_DIR := build
R := scripts/Cargo.toml scripts/Cargo.lock scripts/src/lib.rs

.PHONY: default, run, clean

default: run

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/stage_2.bin: $(R) scripts/src/bin/stage_2.rs stage_2.nasm | $(BUILD_DIR)
	cd scripts && cargo run --bin stage_2

$(BUILD_DIR)/rust.bin: $(R) rust/Cargo.toml rust/Cargo.lock rust/.cargo/config.toml rust/build.rs rust/linker.ld rust/src/main.rs build/stage_2.bin | $(BUILD_DIR)
	cd rust && cargo objcopy --target x86_64-unknown-none -- -O binary ../$(BUILD_DIR)/rust.bin

$(BUILD_DIR)/stage_0.bin: $(R) scripts/src/bin/stage_0.rs stage_0.nasm $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin | $(BUILD_DIR)
	cd scripts && cargo run --bin stage_0

$(BUILD_DIR)/stage_1.bin: $(R) scripts/src/bin/stage_1.rs stage_1.nasm $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin $(BUILD_DIR)/stage_0.bin | $(BUILD_DIR)
	cd scripts && cargo run --bin stage_1

$(BUILD_DIR)/disk.img: $(R) scripts/src/bin/disk.rs $(BUILD_DIR)/stage_0.bin $(BUILD_DIR)/stage_1.bin $(BUILD_DIR)/stage_2.bin $(BUILD_DIR)/rust.bin | $(BUILD_DIR)
	cd scripts && cargo run --bin disk


run: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
        --machine q35,accel=kvm:whpx:hvf:tcg \
        --no-reboot \
        --nographic \
        -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on

clean:
	rm -rf $(BUILD_DIR)

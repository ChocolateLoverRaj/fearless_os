BUILD_DIR := build

.PHONY: default, run, clean

default: run

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/boot.bin: boot.S | $(BUILD_DIR)
	nasm boot.S -f bin -o $(BUILD_DIR)/boot.bin

$(BUILD_DIR)/disk.img: Cargo.toml Cargo.lock src/main.rs $(BUILD_DIR)/boot.bin | $(BUILD_DIR)
	cargo run

run: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
        --machine q35,accel=kvm:whpx:hvf:tcg \
        --no-reboot \
        --nographic \
        -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on
clean:
	rm -rf $(BUILD_DIR)

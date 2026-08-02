BUILD_DIR := build
R := scripts/Cargo.toml scripts/Cargo.lock scripts/src/lib.rs

.PHONY: default, run, run_with_graphic, ndisasm

default: run

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/disk_sector_0.bin: | $(BUILD_DIR)
	nasm ../mbr_bootloader/bootloader.nasm -DMAGIC=0xA786B9FC -f bin -o $(BUILD_DIR)/disk_sector_0.bin

.PHONY: $(BUILD_DIR)/sector_1.bin
$(BUILD_DIR)/sector_1.bin: $(BUILD_DIR)/util.bin | $(BUILD_DIR)
	cd sector_1 && cargo objcopy --target x86_64-unknown-none -- -O binary ../$(BUILD_DIR)/sector_1.bin

.PHONY: $(BUILD_DIR)/sector_1
$(BUILD_DIR)/sector_1: $(BUILD_DIR)/util.bin | $(BUILD_DIR)
	cd sector_1 && cargo build --target x86_64-unknown-none
	ln -sf ../sector_1/target/x86_64-unknown-none/debug/sector_1 $(BUILD_DIR)/sector_1

$(BUILD_DIR)/util.bin $(BUILD_DIR)/util.lst $(BUILD_DIR)/util.map : util.nasm | $(BUILD_DIR)
	nasm util.nasm -f bin -l $(BUILD_DIR)/util.lst -o $(BUILD_DIR)/util.bin

.PHONY: $(BUILD_DIR)/build_sector_0
$(BUILD_DIR)/build_sector_0: | $(BUILD_DIR)
	cd scripts && cargo build --bin sector_0 && cd ..
	ln -sf ../scripts/target/debug/sector_0 $(BUILD_DIR)/build_sector_0

$(BUILD_DIR)/sector_0.bin $(BUILD_DIR)/sector_0.lst: sector_0.nasm $(BUILD_DIR)/build_sector_0 $(BUILD_DIR)/sector_1 $(BUILD_DIR)/util.bin | $(BUILD_DIR)
	$(BUILD_DIR)/build_sector_0

.PHONY: $(BUILD_DIR)/build_disk
$(BUILD_DIR)/build_disk: | $(BUILD_DIR)
	cd scripts && cargo build --bin disk && cd ..
	ln -sf ../scripts/target/debug/disk $(BUILD_DIR)/build_disk

$(BUILD_DIR)/disk.img: $(BUILD_DIR)/disk_sector_0.bin $(BUILD_DIR)/sector_0.bin $(BUILD_DIR)/sector_1.bin $(BUILD_DIR)/build_disk $(BUILD_DIR)/util.bin | $(BUILD_DIR)
	$(BUILD_DIR)/build_disk

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

ndisasm: $(BUILD_DIR)/sector_0.bin
	ndisasm -o 0x7C00 $(BUILD_DIR)/sector_0.bin

clean:
	rm -rf $(BUILD_DIR)

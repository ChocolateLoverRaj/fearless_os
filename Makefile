BUILD_DIR := build
R := scripts/Cargo.toml scripts/Cargo.lock scripts/src/lib.rs

.PHONY: default, run, run_with_graphic, ndisasm

default: run

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/disk_sector_0.bin: | $(BUILD_DIR)
	nasm mbr_bootloader/bootloader.nasm -DMAGIC=0xA786B9FC -f bin -o $(BUILD_DIR)/disk_sector_0.bin

.PHONY: $(BUILD_DIR)/big_stage.bin
$(BUILD_DIR)/big_stage.bin: | $(BUILD_DIR)
	cargo -Z unstable-options -C bios_bootloader_big_stage objcopy --target x86_64-unknown-none -- -O binary ../$(BUILD_DIR)/big_stage.bin

.PHONY: $(BUILD_DIR)/big_stage
$(BUILD_DIR)/big_stage: | $(BUILD_DIR)
	cargo -Z unstable-options -C bios_bootloader_big_stage b --target x86_64-unknown-none
	ln -sf ../target/x86_64-unknown-none/debug/bios_bootloader_big_stage $(BUILD_DIR)/big_stage

.PHONY: $(BUILD_DIR)/small_stage.bin
$(BUILD_DIR)/small_stage.bin: $(BUILD_DIR)/util.bin $(BUILD_DIR)/big_stage | $(BUILD_DIR)
	cargo -Z unstable-options -C bios_bootloader_small_stage objcopy --target x86_64-unknown-none -- -O binary ../$(BUILD_DIR)/small_stage.bin

.PHONY: $(BUILD_DIR)/small_stage
$(BUILD_DIR)/small_stage: $(BUILD_DIR)/util.bin $(BUILD_DIR)/big_stage | $(BUILD_DIR)
	cargo -Z unstable-options -C bios_bootloader_small_stage b --target x86_64-unknown-none
	ln -sf ../target/x86_64-unknown-none/debug/bios_bootloader_small_stage $(BUILD_DIR)/small_stage

#.PHONY: build_scripts
#build_scripts: | $(BUILD_DIR)
#	cd scripts && cargo build && cd ..
#	ln -sf ../scripts/target/debug/sector_0 $(BUILD_DIR)/build_sector_0
#	ln -sf ../scripts/target/debug/disk $(BUILD_DIR)/build_disk
#	ln -sf ../scripts/target/debug/build_util $(BUILD_DIR)/build_util

$(BUILD_DIR)/util.bin $(BUILD_DIR)/util.lst $(BUILD_DIR)/util.map: bios_bootloader_common/util.nasm | $(BUILD_DIR)
	cargo r -p scripts --bin build_util

$(BUILD_DIR)/sector_0.bin $(BUILD_DIR)/sector_0.lst: bios_bootloader_small_stage/sector_0.nasm $(BUILD_DIR)/small_stage $(BUILD_DIR)/util.bin | $(BUILD_DIR)
	cargo r -p scripts --bin build_sector_0

$(BUILD_DIR)/disk.img: $(BUILD_DIR)/disk_sector_0.bin $(BUILD_DIR)/sector_0.bin $(BUILD_DIR)/small_stage.bin $(BUILD_DIR)/util.bin $(BUILD_DIR)/big_stage.bin | $(BUILD_DIR)
	cargo r -p scripts --bin build_disk

run: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
        --machine q35,accel=kvm:whpx:hvf:tcg -d int,cpu_reset -D $(BUILD_DIR)/qemu.log -m 4G \
        --cpu qemu64,+la57 \
        --no-reboot \
        -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on \
        -device usb-ehci,id=ehci \
        -device usb-mouse,bus=ehci.0 \
        -trace "usb_ehci_*" \
        -d trace:pci_cfg_write \
        --nographic

run_with_graphic: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
       --machine q35,accel=kvm:whpx:hvf:tcg -d int,cpu_reset -D $(BUILD_DIR)/qemu.log -m 4G \
       --cpu qemu64,+la57 \
       --no-reboot \
       -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on \
       -serial mon:stdio

ndisasm: $(BUILD_DIR)/sector_0.bin
	ndisasm -o 0x7C00 $(BUILD_DIR)/sector_0.bin

clean:
	rm -rf $(BUILD_DIR)

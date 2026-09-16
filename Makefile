BUILD_DIR := build
R := scripts/Cargo.toml scripts/Cargo.lock scripts/src/lib.rs

.PHONY: default, run_bios, run_bios_with_graphic, ndisasm, run_uefi, run_uefi_with_graphic, run_uefi_usb

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

$(BUILD_DIR)/util.bin $(BUILD_DIR)/util.lst $(BUILD_DIR)/util.map: bios_bootloader_common/util.nasm | $(BUILD_DIR)
	cargo r -p scripts --bin build_util

$(BUILD_DIR)/sector_0.bin $(BUILD_DIR)/sector_0.lst: bios_bootloader_small_stage/sector_0.nasm $(BUILD_DIR)/small_stage $(BUILD_DIR)/util.bin | $(BUILD_DIR)
	cargo r -p scripts --bin build_sector_0

.PHONY: $(BUILD_DIR)/kernel.efi
$(BUILD_DIR)/kernel.efi: | $(BUILD_DIR)
	cargo -Z unstable-options -C kernel_uefi b --target x86_64-unknown-uefi
	ln -sf ../target/x86_64-unknown-uefi/debug/kernel_uefi.efi $(BUILD_DIR)/kernel.efi

$(BUILD_DIR)/disk.img: $(BUILD_DIR)/disk_sector_0.bin $(BUILD_DIR)/sector_0.bin $(BUILD_DIR)/small_stage.bin $(BUILD_DIR)/util.bin $(BUILD_DIR)/big_stage.bin $(BUILD_DIR)/kernel.efi | $(BUILD_DIR)
	cargo r -p scripts --bin build_disk

run_bios: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
        --machine q35,accel=kvm:whpx:hvf:tcg -d int,cpu_reset -D $(BUILD_DIR)/qemu.log -m 4G \
        --no-reboot \
        -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on \
        -device usb-ehci,id=ehci \
        -device usb-mouse,bus=ehci.0 \
        --nographic

run_bios_with_graphic: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
       --machine q35,accel=kvm:whpx:hvf:tcg -d int,cpu_reset -D $(BUILD_DIR)/qemu.log -m 4G \
       --cpu qemu64,+la57 \
       --no-reboot \
       -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on \
       -serial mon:stdio

run_uefi: $(BUILD_DIR)/kernel.efi
	mkdir -p $(BUILD_DIR)/efi_partition/EFI/BOOT
	ln -sf ../../../kernel.efi $(BUILD_DIR)/efi_partition/EFI/BOOT/BOOTX64.EFI
	qemu-system-x86_64 \
       --machine q35,accel=kvm:whpx:hvf:tcg -d int,cpu_reset -D $(BUILD_DIR)/qemu.log -m 4G \
       --no-reboot \
       -drive if=pflash,format=raw,readonly=on,file=$(OVMF_PATH) \
       -drive format=raw,file=fat:rw:$(BUILD_DIR)/efi_partition \
       --nographic \
       -serial none \
       -chardev stdio,id=s1,mux=on \
       -mon chardev=s1,mode=readline \
       -device pci-serial,chardev=s1

run_uefi_with_graphic: $(BUILD_DIR)/kernel.efi
	mkdir -p $(BUILD_DIR)/efi_partition/EFI/BOOT
	ln -sf ../../../kernel.efi $(BUILD_DIR)/efi_partition/EFI/BOOT/BOOTX64.EFI
	qemu-system-x86_64 \
       --machine q35,accel=kvm:whpx:hvf:tcg -d int,cpu_reset -D $(BUILD_DIR)/qemu.log -m 4G \
       --no-reboot \
       -drive if=pflash,format=raw,readonly=on,file=$(OVMF_PATH) \
       -drive format=raw,file=fat:rw:$(BUILD_DIR)/efi_partition \
       -chardev stdio,id=s1,mux=on \
       -mon chardev=s1,mode=readline \
       -device pci-serial,chardev=s1

run_uefi_usb: $(BUILD_DIR)/disk.img
	qemu-system-x86_64 \
       --machine q35,accel=kvm:whpx:hvf:tcg -d int,cpu_reset -D $(BUILD_DIR)/qemu.log -m 4G \
       -drive if=pflash,format=raw,readonly=on,file=$(OVMF_PATH) \
       --no-reboot \
       -drive if=none,id=stick,format=raw,file=$(BUILD_DIR)/disk.img,snapshot=on \
       -device qemu-xhci,id=xhci \
       -device usb-storage,bus=xhci.0,drive=stick,removable=on \
       -drive file=$(BUILD_DIR)/disk.img,format=raw,if=ide,snapshot=on \
       --nographic

ndisasm: $(BUILD_DIR)/sector_0.bin
	ndisasm -o 0x7C00 $(BUILD_DIR)/sector_0.bin

clean:
	rm -rf $(BUILD_DIR)

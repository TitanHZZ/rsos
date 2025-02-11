# debug/release
MODE ?= debug

kernel        := target/kernel.bin
grub_cfg      := src/setup/grub.cfg
linker_script := src/setup/linker.ld

.PHONY: boot kernel all build iso run clean

all: build iso run

# determine the cargo command to use
ifeq ($(MODE), release)
    CARGO_BUILD_CMD = cargo build --release
else
    CARGO_BUILD_CMD = cargo build
endif

# build boot.asm
boot: src/setup/boot.asm
	@echo "Building boot.asm..."
	@mkdir -p target
	@nasm -f elf64 src/setup/boot.asm -o target/boot.o

# compile the rsos kernel into an obj file
kernel:
	@echo "Building rsos kernel..."
	$(CARGO_BUILD_CMD)

# link boot and kernel
build: boot kernel $(linker_script)
	@echo "Linking boot with the kernel..."
	@ld -n -o $(kernel) -T $(linker_script) target/boot.o target/x86_64-rsos/$(MODE)/librsos.a

# create the iso file
iso: $(grub_cfg) $(kernel)
	@echo "Building iso file..."
	@mkdir -p target/isofiles/boot/grub
	@cp $(kernel)  target/isofiles/boot
	@cp $(grub_cfg)  target/isofiles/boot/grub
	@grub2-mkrescue -o target/rsos.iso target/isofiles 2> /dev/null

# run the os in qemu
run: target/rsos.iso
	@echo "Running rsos..."
	@qemu-system-x86_64 -enable-kvm -m 4G -cdrom target/rsos.iso

clean:
	@echo "Cleaning..."
	@rm -f -r target/isofiles
	@rm -f -r target/boot.o
	@rm -f -r target/kernel.bin
	@rm -f -r target/rsos.iso

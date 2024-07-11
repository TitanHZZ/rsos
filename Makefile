kernel        := target/kernel.bin
grub_cfg      := src/setup/grub.cfg
linker_script := src/setup/linker.ld

.PHONY: all build iso run clean

all: build iso run

# build (compile and link) asm file
build: src/setup/boot.asm $(linker_script)
	@echo "Assembling asm file..."
	@mkdir -p target
	@nasm -f elf64 src/setup/boot.asm -o target/boot.o
	@echo "Linking asm file..."
	@ld -n -o $(kernel) -T $(linker_script) target/boot.o

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
	@qemu-system-x86_64 -cdrom target/rsos.iso

clean:
	@echo "Cleaning..."
	@rm -f -r target/isofiles
	@rm -f -r target/boot.o
	@rm -f -r target/kernel.bin
	@rm -f -r target/rsos.iso

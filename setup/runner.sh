#!/bin/sh

mkdir -p target/isofiles/boot/grub

cp "$1" target/isofiles/boot/kernel.bin

cp setup/grub.cfg target/isofiles/boot/grub

grub2-mkrescue -o target/rsos.iso target/isofiles 2> /dev/null

qemu-system-x86_64 -enable-kvm -m 4G -cdrom target/rsos.iso \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 `# this is an I/O device that allows for a simple way to shutdown qemu (useful for tests)` \
    -serial stdio `# this add a serial device (UART) and redirects the output to stdio (so that we can write to the host's terminal)`

ret=$?

# 33 is the success exit code for the tests --> (0x10 << 1) | 1 = 33
if [ $ret -eq 33 ]; then
    # qemu and tests terminated properly
    exit 0
# 35 is the failure exit code for the tests --> (0x11 << 1) | 1 = 35
elif [ $ret -eq 35 ]; then
    # some test failed
    exit 1
elif [ $ret -eq 1 ]; then
    # qemu failed
    exit 2
else
    # unknown return code
    exit 3
fi

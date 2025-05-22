#!/bin/sh

# TODO: add timeouts to the tests so that they do not indefinitely hang

USE_UEFI=0

mkdir -p target/isofiles/boot/grub
cp "$1" target/isofiles/boot/kernel.bin

# check for 'test' mode
if [[ "$1" == *"/deps/"* ]]; then
    cp setup/grub_test.cfg target/isofiles/boot/grub/grub.cfg
else
    cp setup/grub.cfg target/isofiles/boot/grub
fi

grub2-mkrescue -o target/rsos.iso target/isofiles 2> /dev/null

cmd="qemu-system-x86_64 -enable-kvm -m 4G -cdrom target/rsos.iso"

# switch to UEFI mode if requested
if [ $USE_UEFI -eq 1 ]; then
    # make a writable copy of OVMF_VARS.fd
    cp /usr/share/OVMF/OVMF_VARS.fd /tmp/OVMF_VARS.fd

    cmd+=" -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd"
    cmd+=" -drive if=pflash,format=raw,file=/tmp/OVMF_VARS.fd"
fi

# check for 'test' mode
if [[ "$1" == *"/deps/"* ]]; then
    # this is an I/O device that allows for a simple way to shutdown qemu (useful for tests)
    cmd+=" -device isa-debug-exit,iobase=0xf4,iosize=0x04"

    # this add a serial device (UART) and redirects the output to stdio (so that we can write to the host's terminal)
    cmd+=" -serial stdio"

    # hide qemu
    cmd+=" -display none"
fi

eval $cmd
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

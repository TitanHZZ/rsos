#!/bin/sh

mkdir -p target/isofiles/boot/grub

cp "$1" target/isofiles/boot/kernel.bin

cp setup/grub.cfg target/isofiles/boot/grub

grub2-mkrescue -o target/rsos.iso target/isofiles 2> /dev/null

qemu-system-x86_64 -enable-kvm -m 4G -cdrom target/rsos.iso

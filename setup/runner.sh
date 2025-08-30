#!/bin/bash

# globals
TEST_MODE=false        # determines if cargo is running in test mode
TESTS_TIMEOUT="5s"     # timeout for tests (if in test mode)
GRUB_TIMEOUT_RELEASE=5 # grub timeout in cargo release mode
GRUB_TIMEOUT_DEBUG=0   # grub timeout in cargo debug mode

# colors
BRed='\033[1;31m' # bold red
NC='\033[0m'      # no color
BNC='\033[1m'     # bold no color

# check for 'test' mode
if [[ "$1" == *"/deps/"* ]]; then
    TEST_MODE=true
fi

# the base command that will run the OS
# https://wiki.osdev.org/UEFI#Emulation_with_QEMU_and_OVMF
cmd=(
    qemu-system-x86_64 -enable-kvm -m 4G
    -cdrom target/rsos.iso
    -drive if=pflash,format=raw,unit=0,file=/usr/share/OVMF/OVMF_CODE.fd,readonly=on
    -drive if=pflash,format=raw,unit=1,file=/tmp/OVMF_VARS.fd
    -net none
    -no-reboot
)

# make a writable copy of OVMF_VARS.fd for UEFI support
cp /usr/share/OVMF/OVMF_VARS.fd /tmp/OVMF_VARS.fd

# copy the necessary boot files
mkdir -p target/isofiles/boot/grub
cp "$1" target/isofiles/boot/kernel.bin

# copy the grub config file
cp setup/grub.cfg target/isofiles/boot/grub

if $TEST_MODE; then
    # set the appropriate grub timeout
    sed -i "s/GRUB_TIMEOUT/${GRUB_TIMEOUT_DEBUG}/g" target/isofiles/boot/grub/grub.cfg

    # this is an I/O device that allows for a simple way to shutdown qemu (useful for tests)
    cmd+=(-device "isa-debug-exit,iobase=0xf4,iosize=0x04")

    # this add a serial device (UART) and redirects the output to stdio (so that we can write to the host's terminal)
    # cmd+=(-serial stdio)

    # hide qemu
    cmd+=(-display none)
else
    # set the appropriate grub timeout
    sed -i "s/GRUB_TIMEOUT/${GRUB_TIMEOUT_RELEASE}/g" target/isofiles/boot/grub/grub.cfg

    # prevent qemu from shutting down forever in case of an OS crash
    cmd+=(-no-shutdown)
fi

### TEMPORARY
cmd+=(-serial stdio)

grub2-mkrescue -o target/rsos.iso target/isofiles 2> /dev/null

if $TEST_MODE; then
    # run tests with a timeout
    timeout --foreground "$TESTS_TIMEOUT" "${cmd[@]}" | tail -n +3

    # get the exit code for the tests
    status=${PIPESTATUS[0]} # error code from the timeout command

    # 33 is the success exit code for the tests --> (0x10 << 1) | 1 = 33
    if [ $status -eq 33 ]; then
        # qemu and tests terminated properly
        exit 0
    # 35 is the failure exit code for the tests --> (0x11 << 1) | 1 = 35
    elif [ $status -eq 35 ]; then
        # some test failed
        echo -e "${BRed}error${NC}${BNC}:${NC} the test has failed"
        exit 1
    elif [ $status -eq 1 ]; then
        # qemu failed
        echo -e "${BRed}error${NC}${BNC}:${NC} qemu has failed"
        exit 2
    # because of the '-no-reboot' option for the tests, if the OS crashes, qemu just exits with code 0
    elif [ $status -eq 0 ]; then
        # the OS crashed
        echo -e "${BRed}error${NC}${BNC}:${NC} the OS has crashed"
        exit 3
    # 124 is the exit code for the `timeout` command when the command times out
    elif [ $status -eq 124 ]; then
        # the test timed out
        echo -e "${BRed}error${NC}${BNC}:${NC} the test timed out"
        exit 4
    else
        # unknown error
        echo -e "${BRed}error${NC}${BNC}:${NC} an unknown error has occurred"
        exit 5
    fi
else
    # non tests run freely
    "${cmd[@]}" | tail -n +3
fi

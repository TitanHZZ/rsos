# rsos
Simple Rust OS.

## Prerequisites
To run this OS you will have to use [QEMU](https://www.qemu.org/) and [GCC](https://gcc.gnu.org/).  
Please keep in mind that this project was only tested in Fedora with QEMU/KVM and GCC and is known to work in this environment.

## Dependencies
You might need to install `grub2-efi-x64-modules` to have UEFI support.  
Please keep in mind that `grub2-efi-x64-modules` is the package name in Fedora, and might be different in your system.

## Running the OS
To run this:
```bash
cargo run
```

To run tests:
```bash
cargo test
```

## Preview
![Preview Image](./images/preview.png)

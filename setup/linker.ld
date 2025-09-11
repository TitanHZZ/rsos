ENTRY(_start)

SECTIONS
{
	/* Global Symbols */
	KERNEL_LH_START = 16M;
	KERNEL_HH_START = 0xFFFF800000000000;
	KERNEL_LH_HH_OFFSET = KERNEL_HH_START - KERNEL_LH_START;

	/* It used to be universally recommended to use 1M as a start offset,
	   as it was effectively guaranteed to be available under BIOS systems.
	   However, UEFI has made things more complicated, and experimental data
	   strongly suggests that 2M is a safer place to load. In 2016, a new
	   feature was introduced to the multiboot 2 spec to inform bootloaders
	   that a kernel can be loaded anywhere within a range of addresses and
	   will be able to relocate itself to run from such a loader-selected
	   address, in order to give the loader freedom in selecting a span of
	   memory which is verified to be available by the firmware, in order to
	   work around this issue. This does not use that feature, so a safer
	   option was chosen rather than the traditional 1M or even 2M. */
	. = KERNEL_LH_START;

	/* First put the multiboot 2 header, as it is required to be put very early
	   in the image or the bootloader won't recognize the file format, and
	   then, group all the text (code) sections for the lower half. */
	.text.lower_half : AT(KERNEL_LH_START)
	{
		KEEP(*(.multiboot_header))
		*(.text.lower_half .text.lower_half.*)
		. = ALIGN(4K);
	}

	.bss.lower_half : AT(LOADADDR(.text.lower_half) + SIZEOF(.text.lower_half))
	{
		KEEP(*(.bss.lower_half .bss.lower_half.*))
		. = ALIGN(4K);
	}

	.rodata.lower_half : AT(LOADADDR(.bss.lower_half) + SIZEOF(.bss.lower_half))
	{
		KEEP(*(.rodata.lower_half .rodata.lower_half.*))
		. = ALIGN(4K);
	}

	/* All the sections before this are essentially the initial bootstrap for the kernel,
	   and all the sections after, are the kernel itself. */
	. = KERNEL_HH_START + (. - KERNEL_LH_START);

	/* Group all the higher half text (code) sections. */
	.text : AT(LOADADDR(.rodata.lower_half) + SIZEOF(.rodata.lower_half))
	{
		/* group together all the text sections made by the rust compiler */
		*(.text .text.*)
		*(.ltext .ltext.*)
		. = ALIGN(4K);
	}

	/* Read-only data. */
	.rodata : AT(LOADADDR(.text) + SIZEOF(.text))
	{
		/* group together all the rodata sections made by the rust compiler */
		KEEP(*(.rodata .rodata.*))
		KEEP(*(.lrodata .lrodata.*))
		. = ALIGN(4K);
	}

    /* Group more rust compiler sections. */
	.data.rel.ro : AT(LOADADDR(.rodata) + SIZEOF(.rodata))
    {
		KEEP(*(.data.rel.ro .data.rel.ro.* .data.rel.ro.local*))
		KEEP(*(.ldata.rel.ro .ldata.rel.ro.* .ldata.rel.ro.local*))
		. = ALIGN(4K);
    }

	/* Read-write data (initialized). */
	.data : AT(LOADADDR(.data.rel.ro) + SIZEOF(.data.rel.ro))
	{
		KEEP(*(.data .data.*))
		KEEP(*(.ldata .ldata.*))
		. = ALIGN(4K);
	}

	/* Read-write data (uninitialized) and stack. */
	.bss : AT(LOADADDR(.data) + SIZEOF(.data))
	{
		KEEP(*(COMMON))
		KEEP(*(.bss .bss.*))
		KEEP(*(.lbss .lbss.*))
		. = ALIGN(4K);
	}

	/* Exception Frames. */
	.eh_frame : AT(LOADADDR(.bss) + SIZEOF(.bss))
	{
		*(.eh_frame .eh_frame.*)
		. = ALIGN(4K);
	}

	/* Debug GDB Scripts. */
	.debug_gdb_scripts : AT(LOADADDR(.eh_frame) + SIZEOF(.eh_frame))
	{
		*(.debug_gdb_scripts)
		. = ALIGN(4K);
	}

	/* Other sections may be produced, but they *shouldn't* be necessary. */
	/DISCARD/ :
	{
		*(*)
	}
}

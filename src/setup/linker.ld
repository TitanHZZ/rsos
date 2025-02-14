ENTRY(_start)

PHDRS
{
	r PT_LOAD FLAGS(R);
	rw PT_LOAD FLAGS(RW);
	rx PT_LOAD FLAGS(RX);
}

SECTIONS
{
	/* It used to be universally recommended to use 1M as a start offset,
	   as it was effectively guaranteed to be available under BIOS systems.
	   However, UEFI has made things more complicated, and experimental data
	   strongly suggests that 2M is a safer place to load. In 2016, a new
	   feature was introduced to the multiboot 2 spec to inform bootloaders
	   that a kernel can be loaded anywhere within a range of addresses and
	   will be able to relocate itself to run from such a loader-selected
	   address, in order to give the loader freedom in selecting a span of
	   memory which is verified to be available by the firmware, in order to
	   work around this issue. This does not use that feature, so 2M was
	   chosen as a safer option than the traditional 1M. */
	. = 2M;

	/* First put the multiboot 2 header, as it is required to be put very early
	   in the image or the bootloader won't recognize the file format,
	   and the, group all text (code) sections */
	.text BLOCK(4K) : ALIGN(4K)
	{
		*(.multiboot_header)
		*(.text .text.*) /* group together all the text sections made by the rust compiler */
	} : rx

	/* Read-only data */
	.rodata BLOCK(4K) : ALIGN(4K)
	{
		*(.rodata .rodata.*) /* group together all the rodata sections made by the rust compiler */
	} : r

    /* group more rust compiler sections */
	.data.rel.ro BLOCK(4K) : ALIGN(4K)
	{
        *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    } : r

	/* Read-write data (initialized) */
	.data BLOCK(4K) : ALIGN(4K)
	{
		*(.data .data.*)
	} : rw

	/* Read-write data (uninitialized) and stack */
	.bss BLOCK(4K) : ALIGN(4K)
	{
		*(COMMON)
		*(.bss .bss.*)
	} : rw

	/* Global Offset Table */
	.got BLOCK(4K) : ALIGN(4K)
	{
		*(.got)
	}

	/* Procedure Linkage Table */
	.got.plt BLOCK(4K) : ALIGN(4K)
	{
		*(.got.plt)
	}

	/* Exception Frames */
	.eh_frame BLOCK(4K) : ALIGN(4K)
	{
		*(.eh_frame .eh_frame.*)
	}

	/* Debug GDB Scripts */
	.debug_gdb_scripts BLOCK(4K) : ALIGN(4K)
	{
		*(.debug_gdb_scripts)
	}

	/* The compiler may produce other sections, by default it will put them in
	   a segment with the same name.*/
}

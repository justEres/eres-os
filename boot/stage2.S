.intel_syntax noprefix

.section .text.boot
.code16
.global _start
.extern kernel_main

.equ CODE32_SEL, 0x08
.equ DATA_SEL,   0x10
.equ CODE64_SEL, 0x18

_start:
    cli
    mov al, 'S'
    out 0xe9, al

    lgdt [gdt_descriptor]

    mov eax, cr0
    or eax, 0x1
    mov cr0, eax
    ljmp CODE32_SEL, protected_mode_entry

.code32
protected_mode_entry:
    mov ax, DATA_SEL
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, OFFSET FLAT:stack_top

    mov eax, OFFSET FLAT:pml4_table
    mov cr3, eax

    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr

    mov eax, cr0
    or eax, (1 << 31)
    mov cr0, eax

    ljmp CODE64_SEL, long_mode_entry

.code64
long_mode_entry:
    mov al, 'L'
    out 0xe9, al

    mov ax, DATA_SEL
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov rsp, OFFSET FLAT:stack_top

    mov al, 'R'
    out 0xe9, al

    call kernel_main

1:
    hlt
    jmp 1b

.section .text.boot
.align 8
gdt_start:
    .quad 0x0000000000000000
    .quad 0x00CF9A000000FFFF
    .quad 0x00CF92000000FFFF
    .quad 0x00AF9A000000FFFF
gdt_end:

gdt_descriptor:
    .word gdt_end - gdt_start - 1
    .long gdt_start

.section .data
.align 4096
pml4_table:
    .quad pdpt_table + 0x03
    .fill 511, 8, 0

.align 4096
pdpt_table:
    .quad pd_table + 0x03
    .fill 511, 8, 0

.align 4096
pd_table:
    .quad 0x0000000000000083
    .fill 511, 8, 0

.align 16
stack_bottom:
    .fill 4096, 1, 0
stack_top:

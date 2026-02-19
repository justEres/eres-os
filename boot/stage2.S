.intel_syntax noprefix

.section .text.boot
.code16
.global _start
.extern kernel_main

.equ CODE32_SEL, 0x08
.equ DATA_SEL,   0x10
.equ CODE64_SEL, 0x18
.equ BOOT_INFO_ADDR, 0x4f00
.equ E820_BUFFER_ADDR, 0x5000
.equ E820_MAX_ENTRIES, 64
.equ BOOT_INFO_MAGIC, 0x534f5245

_start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7b00

    mov al, 'S'
    out 0xe9, al

    call detect_memory_map

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

    mov rdi, BOOT_INFO_ADDR
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

.section .text.boot
.code16
detect_memory_map:
    mov dword ptr [BOOT_INFO_ADDR + 0], BOOT_INFO_MAGIC
    mov dword ptr [BOOT_INFO_ADDR + 4], 1
    mov dword ptr [BOOT_INFO_ADDR + 8], 0
    mov dword ptr [BOOT_INFO_ADDR + 12], 24
    mov dword ptr [BOOT_INFO_ADDR + 16], E820_BUFFER_ADDR
    mov dword ptr [BOOT_INFO_ADDR + 20], 0

    mov word ptr [e820_count], 0
    mov word ptr [e820_ptr], E820_BUFFER_ADDR

    xor ebx, ebx

1:
    mov eax, 0xE820
    mov ecx, 24
    mov edx, 0x534D4150
    mov di, word ptr [e820_ptr]
    int 0x15
    jc 2f
    cmp eax, 0x534D4150
    jne 2f

    mov ax, word ptr [e820_count]
    inc ax
    mov word ptr [e820_count], ax

    add word ptr [e820_ptr], 24
    cmp ax, E820_MAX_ENTRIES
    jae 2f

    test ebx, ebx
    jne 1b

2:
    xor eax, eax
    mov ax, word ptr [e820_count]
    mov dword ptr [BOOT_INFO_ADDR + 8], eax
    ret

e820_count:
    .word 0
e820_ptr:
    .word 0

.intel_syntax noprefix
.code16
.section .text
.global _start

.ifndef STAGE2_SECTORS
.set STAGE2_SECTORS, 32
.endif

_start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7c00

    mov byte ptr [boot_drive], dl
    mov al, 'B'
    out 0xe9, al

    mov word ptr [remaining_sectors], STAGE2_SECTORS
    mov word ptr [load_segment], 0x0800
    mov dword ptr [current_lba], 1
    mov dword ptr [current_lba + 4], 0

load_loop:
    mov ax, word ptr [remaining_sectors]
    test ax, ax
    jz load_done

    cmp ax, 127
    jbe count_set
    mov ax, 127
count_set:
    mov word ptr [dap_count], ax
    mov bx, word ptr [load_segment]
    mov word ptr [dap_segment], bx

    mov ax, word ptr [current_lba]
    mov word ptr [dap_lba], ax
    mov ax, word ptr [current_lba + 2]
    mov word ptr [dap_lba + 2], ax
    mov ax, word ptr [current_lba + 4]
    mov word ptr [dap_lba + 4], ax
    mov ax, word ptr [current_lba + 6]
    mov word ptr [dap_lba + 6], ax

    lea si, [disk_address_packet]
    mov ah, 0x42
    mov dl, byte ptr [boot_drive]
    int 0x13
    jc disk_error

    mov ax, word ptr [dap_count]
    shl ax, 5
    add word ptr [load_segment], ax

    mov ax, word ptr [dap_count]
    sub word ptr [remaining_sectors], ax

    xor dx, dx
    add word ptr [current_lba], ax
    adc word ptr [current_lba + 2], dx
    adc word ptr [current_lba + 4], dx
    adc word ptr [current_lba + 6], dx
    jmp load_loop

load_done:

    mov al, '2'
    out 0xe9, al

    ljmp 0x0000, 0x8000

disk_error:
    mov al, 'E'
    out 0xe9, al
    cli
1:
    hlt
    jmp 1b

boot_drive:
    .byte 0x00

remaining_sectors:
    .word 0

load_segment:
    .word 0x0800

current_lba:
    .quad 1

disk_address_packet:
    .byte 0x10
    .byte 0x00
dap_count:
    .word 0
dap_offset:
    .word 0x0000
dap_segment:
    .word 0x0800
dap_lba:
    .quad 1

.org 510
.word 0xaa55

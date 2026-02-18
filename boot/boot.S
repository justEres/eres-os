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

    mov bx, 0x8000

    mov ah, 0x02
    mov al, STAGE2_SECTORS
    mov ch, 0x00
    mov cl, 0x02
    mov dh, 0x00
    mov dl, byte ptr [boot_drive]
    int 0x13
    jc disk_error

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

.org 510
.word 0xaa55

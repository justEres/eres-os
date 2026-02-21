# Stage-1 Bootsektor (16-Bit Real Mode).
# -----------------------------------------------------------------------------
# Kontext:
# - BIOS lädt genau 512 Bytes nach 0000:7C00 und springt dorthin.
# - In diese 512 Bytes passt kein komplexer Long-Mode-Setupcode.
# - Deshalb lädt Stage 1 nur Stage 2 nach und übergibt dann.
# # Warum diese Struktur nötig ist:
# - BIOS-Bootsignatur muss bei Byte 510/511 stehen (0xAA55).
# - Bootsektor muss extrem klein bleiben.
# - Moderne Datenträgerzugriffe erfolgen zuverlässig per INT 13h Extensions
#   (AH=42h, Disk Address Packet / LBA), nicht mehr über CHS.
# # Referenzen:
# - https://wiki.osdev.org/Bootloader
# - https://wiki.osdev.org/BIOS
# - https://wiki.osdev.org/Disk_access_using_the_BIOS_(INT_13h)

.intel_syntax noprefix
.code16
.section .text
.global _start
# Fallback: wenn beim Build keine Sektorzahl übergeben wurde, lädt Stage 1
# pauschal 32 Sektoren (16 KiB) für Stage 2.
.ifndef STAGE2_SECTORS
.set STAGE2_SECTORS, 32
.endif

_start:
# Während Segment-/Stack-Setup dürfen keine Interrupts stören.
    cli
# DS/ES/SS auf 0 setzen -> konsistenter Segmentzustand im Real Mode.
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
# Stack auf 0000:7C00 legen. Das ist nahe beim geladenen Bootcode und
# für diesen kurzen Stage-1-Ablauf ausreichend.
    mov sp, 0x7c00
# BIOS übergibt das Boot-Laufwerk in DL (z. B. 0x80 für erste HDD).
# Wir speichern es, da BIOS-Aufrufe später wieder DL benötigen.
    mov byte ptr [boot_drive], dl
# Debug-Marker für Port 0xE9 (in QEMU/Bochs gut sichtbar):
# 'B' = Stage 1 wurde gestartet.
    mov al, 'B'
    out 0xe9, al
# Initialwerte für den Lade-Loop:
# - remaining_sectors: wie viel Stage2 noch geladen werden muss
# - load_segment: Zielsegment im RAM (0x0800 => physisch 0x8000)
# - current_lba: Start-LBA (1, weil LBA 0 der Bootsektor selbst ist)
    mov word ptr [remaining_sectors], STAGE2_SECTORS
    mov word ptr [load_segment], 0x0800
    mov dword ptr [current_lba], 1
    mov dword ptr [current_lba + 4], 0

load_loop:
# Prüfen, ob wir alle gewünschten Sektoren geladen haben.
    mov ax, word ptr [remaining_sectors]
    test ax, ax
    jz load_done
# INT 13h AH=42h erlaubt pro Aufruf max. 127 Sektoren.
# Daher ggf. in Chunks laden.
    cmp ax, 127
    jbe count_set
    mov ax, 127
count_set:
# Anzahl Sektoren im DAP eintragen.
    mov word ptr [dap_count], ax
# Zielsegment im DAP eintragen. Offset bleibt 0.
    mov bx, word ptr [load_segment]
    mov word ptr [dap_segment], bx
# Aktuelle 64-Bit-LBA in den DAP kopieren (wordweise).
    mov ax, word ptr [current_lba]
    mov word ptr [dap_lba], ax
    mov ax, word ptr [current_lba + 2]
    mov word ptr [dap_lba + 2], ax
    mov ax, word ptr [current_lba + 4]
    mov word ptr [dap_lba + 4], ax
    mov ax, word ptr [current_lba + 6]
    mov word ptr [dap_lba + 6], ax
# SI zeigt auf den DAP, AH=42h wählt Extended Read.
    lea si, [disk_address_packet]
    mov ah, 0x42
# DL muss das Bootlaufwerk enthalten.
    mov dl, byte ptr [boot_drive]
# BIOS-Leseaufruf.
    int 0x13
# Carry gesetzt => Fehler beim Lesen.
    jc disk_error
# Nächste Zieladresse berechnen:
# 1 Sektor = 512 Bytes = 32 Paragraphen (16-Byte-Einheiten).
# Deshalb: load_segment += dap_count * 32.
    mov ax, word ptr [dap_count]
    shl ax, 5
    add word ptr [load_segment], ax
# Verbleibende Sektoren herunterzählen.
    mov ax, word ptr [dap_count]
    sub word ptr [remaining_sectors], ax
# LBA um die geladene Anzahl erhöhen (64-Bit mit Carry-Kette).
    xor dx, dx
    add word ptr [current_lba], ax
    adc word ptr [current_lba + 2], dx
    adc word ptr [current_lba + 4], dx
    adc word ptr [current_lba + 6], dx
    jmp load_loop

load_done:
# Debug-Marker: '2' = Stage 2 liegt im RAM.
    mov al, '2'
    out 0xe9, al
# Far jump in geladenen Stage-2-Code bei 0000:8000.
# Far jump ist wichtig, damit CS explizit gesetzt wird.
    ljmp 0x0000, 0x8000

disk_error:
# Debug-Marker: 'E' = Diskfehler.
    mov al, 'E'
    out 0xe9, al
# Im Fehlerfall sicher anhalten.
    cli
1:
    hlt
    jmp 1b
# ----------------------------------------------------------------------------
# Datenbereich
# ----------------------------------------------------------------------------
# Bootlaufwerk aus DL (BIOS-Konvention).
boot_drive:
    .byte 0x00
# Noch zu ladende Sektoren.
remaining_sectors:
    .word 0
# Zielsegment für den nächsten BIOS-Read.
load_segment:
    .word 0x0800
# Aktuelle 64-Bit-LBA.
current_lba:
    .quad 1
# Disk Address Packet (INT 13h Extensions, AH=42h):
# https://wiki.osdev.org/Disk_access_using_the_BIOS_(INT_13h)#LBA_in_Extended_Mode
# Layout:
#   +0  size (0x10)
#   +1  reserved
#   +2  count
#   +4  offset
#   +6  segment
#   +8  LBA (64 Bit)
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
# BIOS-Bootsignatur an Offset 510/511.
.org 510
.word 0xaa55

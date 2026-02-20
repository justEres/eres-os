; Stage 2 Bootloader (Real Mode -> Protected Mode -> Long Mode -> Rust Kernel)
; -----------------------------------------------------------------------------
; Diese Datei macht den eigentlichen CPU-Moduswechsel und bereitet eine minimale
; Umgebung vor, damit `kernel_main` in 64 Bit laufen kann.
;
; Warum das nötig ist:
; - BIOS startet immer im 16-Bit-Real-Mode.
; - Rust-Kernelcode hier ist für x86_64 gedacht.
; - Daher müssen wir GDT, Paging und Long-Mode-Bits korrekt setzen.
;
; Referenzen:
; - https://wiki.osdev.org/Setting_Up_Long_Mode
; - https://wiki.osdev.org/Protected_Mode
; - https://wiki.osdev.org/Page_table
; - https://wiki.osdev.org/Detecting_Memory_(x86)

.intel_syntax noprefix

.section .text.boot
.code16
.global _start
.extern kernel_main

; Segmentselektoren passend zur GDT weiter unten.
.equ CODE32_SEL, 0x08
.equ DATA_SEL,   0x10
.equ CODE64_SEL, 0x18

; Feste Speicheradressen für die Übergabestruktur und E820-Liste.
.equ BOOT_INFO_ADDR, 0x4f00
.equ E820_BUFFER_ADDR, 0x5000
.equ E820_MAX_ENTRIES, 64
.equ BOOT_INFO_MAGIC, 0x534f5245

_start:
    ; Interrupts aus: während Moduswechsel vermeiden wir asynchrone Ereignisse.
    cli

    ; Segmente auf 0, konsistenter Ausgangszustand im Real Mode.
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; Separater Stage-2-Stack unterhalb 0x7C00.
    mov sp, 0x7b00

    ; Debug-Marker 'S' = Stage 2 gestartet.
    mov al, 'S'
    out 0xe9, al

    ; BIOS-E820-Speicherkarte einsammeln und in BOOT_INFO_ADDR ablegen.
    call detect_memory_map

    ; GDT laden (enthält 32-Bit-Code, Daten und 64-Bit-Code Segment).
    lgdt [gdt_descriptor]

    ; Protected Mode aktivieren: CR0.PE (Bit 0) setzen.
    mov eax, cr0
    or eax, 0x1
    mov cr0, eax

    ; Far jump lädt neues CS aus GDT und leert Prefetch-Queue.
    ljmp CODE32_SEL, protected_mode_entry

.code32
protected_mode_entry:
    ; Ab hier 32-Bit-Modus mit flachem Datensegment.
    mov ax, DATA_SEL
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, OFFSET FLAT:stack_top

    ; PML4-Basis in CR3 setzen (Paging-Hierarchie vorbereiten).
    mov eax, OFFSET FLAT:pml4_table
    mov cr3, eax

    ; CR4.PAE (Bit 5) muss für Long Mode gesetzt sein.
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    ; IA32_EFER MSR lesen und LME (Bit 8) setzen.
    ; MSR 0xC0000080 = EFER.
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr

    ; CR0.PG (Bit 31) aktivieren -> Paging an.
    ; Mit PAE + LME führt der nächste kompatible Sprung in Long Mode.
    mov eax, cr0
    or eax, (1 << 31)
    mov cr0, eax

    ; Far jump in 64-Bit-Code-Segment.
    ljmp CODE64_SEL, long_mode_entry

.code64
long_mode_entry:
    ; Debug-Marker 'L' = Long Mode erreicht.
    mov al, 'L'
    out 0xe9, al

    ; Datensegmente im 64-Bit-Modus neu setzen.
    mov ax, DATA_SEL
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; 64-Bit-Stackpointer setzen.
    mov rsp, OFFSET FLAT:stack_top

    ; Debug-Marker 'R' = direkt vor Rust-Aufruf.
    mov al, 'R'
    out 0xe9, al

    ; SysV x86_64 ABI: 1. Argument in RDI.
    ; Wir übergeben Pointer auf unsere BootInfo-Struktur.
    mov rdi, BOOT_INFO_ADDR
    call kernel_main

    ; Falls kernel_main jemals zurückkehrt: CPU anhalten.
1:
    hlt
    jmp 1b

.section .text.boot
.align 8
gdt_start:
    ; Null-Deskriptor (muss immer der erste Eintrag sein).
    .quad 0x0000000000000000
    ; 32-Bit Code Segment: base=0 limit=4GiB, ausführbar, ring0.
    .quad 0x00CF9A000000FFFF
    ; 32-Bit Data Segment: base=0 limit=4GiB, read/write, ring0.
    .quad 0x00CF92000000FFFF
    ; 64-Bit Code Segment (L-Bit gesetzt).
    .quad 0x00AF9A000000FFFF
gdt_end:

gdt_descriptor:
    .word gdt_end - gdt_start - 1
    .long gdt_start

.section .data
.align 4096
pml4_table:
    ; Erste PML4-Entry zeigt auf die PDPT, flags: Present + Writable (0x3).
    .quad pdpt_table + 0x03
    .fill 511, 8, 0

.align 4096
pdpt_table:
    ; Erste PDPT-Entry zeigt auf die PD, flags: Present + Writable.
    .quad pd_table + 0x03
    .fill 511, 8, 0

.align 4096
pd_table:
    ; Erste PD-Entry: 2MiB Identity-Mapping ab physischer Adresse 0.
    ; Flags 0x83 = Present + Writable + PageSize(2MiB).
    .quad 0x0000000000000083
    .fill 511, 8, 0

.align 16
stack_bottom:
    .fill 4096, 1, 0
stack_top:

.section .text.boot
.code16
detect_memory_map:
    ; BootInfo-Struktur initialisieren, damit Rust sie validieren kann.
    mov dword ptr [BOOT_INFO_ADDR + 0], BOOT_INFO_MAGIC
    mov dword ptr [BOOT_INFO_ADDR + 4], 1

    ; entries wird nach der Schleife gesetzt.
    mov dword ptr [BOOT_INFO_ADDR + 8], 0

    ; Größe eines E820-Eintrags (24 Bytes).
    mov dword ptr [BOOT_INFO_ADDR + 12], 24

    ; Pointer auf den E820-Buffer.
    mov dword ptr [BOOT_INFO_ADDR + 16], E820_BUFFER_ADDR
    mov dword ptr [BOOT_INFO_ADDR + 20], 0

    ; Interne Laufvariablen.
    mov word ptr [e820_count], 0
    mov word ptr [e820_ptr], E820_BUFFER_ADDR

    ; EBX=0 signalisiert "erste E820-Abfrage".
    xor ebx, ebx

1:
    ; BIOS E820 aufrufen: EAX=0xE820, EDX='SMAP', ECX=Buffergröße.
    mov eax, 0xE820
    mov ecx, 24
    mov edx, 0x534D4150
    mov di, word ptr [e820_ptr]
    int 0x15

    ; Carry=1 => kein weiterer/gültiger Eintrag.
    jc 2f

    ; BIOS muss Signatur 'SMAP' zurückgeben.
    cmp eax, 0x534D4150
    jne 2f

    ; Erfolgreichen Eintrag zählen.
    mov ax, word ptr [e820_count]
    inc ax
    mov word ptr [e820_count], ax

    ; Nächstes Ziel im Buffer.
    add word ptr [e820_ptr], 24

    ; Sicherheitsgrenze, damit wir den Buffer nicht überschreiben.
    cmp ax, E820_MAX_ENTRIES
    jae 2f

    ; BIOS liefert Continuation-Wert in EBX.
    ; EBX!=0 => weitere Einträge vorhanden.
    test ebx, ebx
    jne 1b

2:
    ; Anzahl gefundener Einträge in BootInfo schreiben.
    xor eax, eax
    mov ax, word ptr [e820_count]
    mov dword ptr [BOOT_INFO_ADDR + 8], eax
    ret

e820_count:
    .word 0
e820_ptr:
    .word 0

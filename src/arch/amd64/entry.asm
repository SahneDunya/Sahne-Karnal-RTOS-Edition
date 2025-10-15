# src/arch/amd64/entry.s
# AMD64 (x86_64) Mimarisi için Çekirdek Giriş Noktası Kodu.
# Multiboot2 uyumlu önyükleme ve 64-bit Long Mode geçişi.

# --------------------------------------------------------------------------------
# 1. Sabitler ve Multiboot2 Başlığı
# --------------------------------------------------------------------------------

.set ALIGN,    1<<0             # Çekirdek 4KB sınırına hizalanmalı
.set MEMINFO,  1<<1             # Önyükleyici hafıza haritası sağlamalı
.set FLAGS,    ALIGN | MEMINFO  # Başlık Bayrakları
.set MAGIC,    0xE85250D6       # Multiboot2 Başlık Sihir Numarası
.set CHECKSUM, -(MAGIC + FLAGS) # Kontrol Toplamı (CHECKSUM + MAGIC + FLAGS = 0)

.section .multiboot_header
.align 8
# Multiboot2 Başlığı
.long MAGIC
.long FLAGS
.long CHECKSUM
.long . - .multiboot_header      # Başlık Uzunluğu
.long 0                          # Ayrılmış (Reserved)

# --------------------------------------------------------------------------------
# 2. GDT (Global Descriptor Table) Kurulumu
# --------------------------------------------------------------------------------

.section .rodata
.align 8

# GDT Tanımlayıcıları
GDT_NULL:
.quad 0x0

# Code Segment Descriptor (64-bit, L=1)
GDT_CODE_64:
.quad 0x00209a0000000000

# Data Segment Descriptor (32-bit, L=0, D/B=1)
GDT_DATA_32:
.quad 0x00c0920000000000

# GDT Pointer (lgdt talimatı için)
GDT_POINTER:
.word GDT_POINTER - GDT_NULL - 1 # GDT Uzunluğu
.quad GDT_NULL                   # GDT Başlangıç Adresi

# Segment Seçiciler (8 bayt ofset)
.set CODE_SEG, GDT_CODE_64 - GDT_NULL
.set DATA_SEG, GDT_DATA_32 - GDT_NULL

# --------------------------------------------------------------------------------
# 3. Sayfalama (Paging) Yapısı (Minimal Identity Map)
# --------------------------------------------------------------------------------

.section .bss
.align 4096

# Sayfa Üst Dizin Tablosu (PML4)
pml4: .skip 4096

# Sayfa Dizin İşaretçisi Tablosu (PDPT)
pdpt: .skip 4096

# Yığın (Stack)
.equ STACK_SIZE, 0x4000 # 16KB geçici yığın
stack_top: .skip STACK_SIZE
stack_bottom:

# --------------------------------------------------------------------------------
# 4. Giriş Noktası (32-bit Protected Mode Başlangıcı)
# --------------------------------------------------------------------------------

.section .text
.global _start
.type _start, @function
_start:
    # rbx: Multiboot Bilgi Yapısı Adresi
    # eax: Multiboot Magic Number (0x36D76289 - Multiboot2 için)
    # Multiboot2 uyumluluğu için eax=0 kontrolü (GRUB2'nin Multiboot2 davranışına güvenerek)

    cmp $0, %eax
    jne multiboot_error

    # -----------------------------------
    # 4.1. 32-bit'te Sayfalama Kurulumu (Identity Map)
    # -----------------------------------

    # pml4[0] = &pdpt | Present | R/W
    movl $pdpt + 0x3, pml4 

    # pdpt[0] = 1GB Huge Page | Present | R/W | PS (Page Size)
    # 0x0000000000000083 = Present | R/W | Huge Page (1GB)
    movl $0x00000083, pdpt         

    # CR4'te PAE (Physical Address Extension) etkinleştir (bit 5)
    movl %cr4, %eax
    orl $0x20, %eax                 
    movl %eax, %cr4

    # CR3'e PML4'ün fiziksel adresini yükle
    movl $pml4, %eax
    movl %eax, %cr3

    # CR0'da Sayfalama (PG - bit 31) ve Koruma (PE - bit 0) bayraklarını etkinleştir
    movl %cr0, %eax
    orl $(1<<31) | 0x1, %eax        
    movl %eax, %cr0

    # -----------------------------------
    # 4.2. GDT Yükleme ve 64-bit Etkinleştirme
    # -----------------------------------

    # GDT'yi yükle
    lgdt GDT_POINTER

    # EFER'de LME (Long Mode Enable) etkinleştir (MSR 0xC0000080)
    movl $0xc0000080, %ecx          # EFER MSR adresi
    rdmsr                           # EAX:EDX'e oku
    orl $0x100, %eax                # LME bayrağı (bit 8)
    wrmsr                           # EAX:EDX'i MSR'ye yaz

    # Long jump to flush the instruction cache and enter 64-bit mode
    # JMP Segment Seçici:Ofset
    ljmp $CODE_SEG, $long_mode_entry

# --------------------------------------------------------------------------------
# 5. 64-bit Giriş Noktası (Long Mode)
# --------------------------------------------------------------------------------

.code64
.align 8
long_mode_entry:
    # Buradayız: 64-bit Long Mode etkin.
    
    # Veri segmentlerini yeniden yükle (64-bit segment seçici ile)
    mov $DATA_SEG, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    # -----------------------------------
    # 5.1. Yığın Kurulumu
    # -----------------------------------
    
    # 64-bit yığını ayarla (RSP)
    leaq stack_bottom, %rsp

    # -----------------------------------
    # 5.2. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Multiboot Bilgisini (rbx) ve Magic'i (rax) Rust'a geçir.
    # C ABI: RDI (arg 1), RSI (arg 2)
    mov %rbx, %rsi  # Multiboot Yapı Adresi -> RSI
    mov %rax, %rdi  # 0 -> RDI

    # Rust çekirdeğine kontrolü ver.
    call kernel_main

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
    cli # Kesmeleri kapat
hang:
    hlt # İşlemciyi durdur
    jmp hang

# --------------------------------------------------------------------------------
# 6. Hata Döngüsü
# --------------------------------------------------------------------------------

.global multiboot_error
multiboot_error:
    cli
error_hang:
    hlt
    jmp error_hang

.size _start, . - _start
# src/arch/amd64/boot.s
# AMD64 (x86_64) Mimarisi için Başlangıç (Boot) Kodu.
# Multiboot2 uyumlu önyükleme.

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
# Multiboot2 Başlığı (Multiboot2 Standardı)
.long MAGIC
.long FLAGS
.long CHECKSUM
.long . - .multiboot_header      # Başlık Uzunluğu
.long 0                          # Ayrılmış (Reserved)

# --------------------------------------------------------------------------------
# 2. GDT (Global Descriptor Table) Kurulumu
#    Çekirdeği 64-bit Long Mode'a geçirmek için gereklidir.
# --------------------------------------------------------------------------------

.section .rodata
.align 8

# GDT Tanımlayıcıları
# Null Descriptor
GDT_NULL:
.quad 0x0

# Code Segment Descriptor (64-bit) - Base 0, Limit 0xFFFFF, G=1, D/B=0, L=1, P=1
# D/B=0: 32-bit/16-bit operand varsayımı (L=1 olduğunda 64-bit için zorunlu 0)
# L=1: 64-bit kod segmenti
GDT_CODE_64:
.quad 0x00209a0000000000

# Data Segment Descriptor (Writable) - Base 0, Limit 0xFFFFF, G=1, D/B=1, L=0, P=1
# Data segmenti 32-bit varsayılır (L=0). 64-bit modda segmentleme yapılmadığı için 
# bu değerler genellikle yoksayılır, ancak uyumluluk için gereklidir.
GDT_DATA_32:
.quad 0x00c0920000000000

# GDT Pointer (lgdt talimatı için)
GDT_POINTER:
.word GDT_POINTER - GDT_NULL - 1 # GDT Uzunluğu (son adres - başlangıç adresi)
.quad GDT_NULL                   # GDT Başlangıç Adresi

# Segment Seçiciler
.set CODE_SEG, GDT_CODE_64 - GDT_NULL
.set DATA_SEG, GDT_DATA_32 - GDT_NULL

# --------------------------------------------------------------------------------
# 3. Sayfalama (Paging) Yapısı (Minimal 4-seviye)
# --------------------------------------------------------------------------------

.section .bss

.align 4096
# Sayfa Üst Dizin Tablosu (PML4 - 512 giriş * 8 byte/giriş)
pml4: .skip 4096

# Sayfa Dizin İşaretçisi Tablosu (PDPT)
# Identity map (0-1GB) için tek bir PDPT yeterlidir.
pdpt: .skip 4096

# Sayfa Dizin Tablosu (PD - 512 giriş * 8 byte/giriş)
# 1GB'lık ilk bölgeyi eşleştirmek için tek bir 1GB sayfa tablosu (2MB sayfalarla)
# veya 512 adet 2MB sayfa girişi (2MB * 512 = 1GB) gereklidir.
# En basit: 1GB'lık dev bir sayfa (Huge Page) kullanmak (PDPT'de yapılır).
# VEYA: İlk 2MB'ı eşleştirmek için bir PD (512 * 4KB = 2MB).
pd: .skip 4096

# Yığın (Stack)
.equ STACK_SIZE, 0x4000 # 16KB geçici yığın
stack_top: .skip STACK_SIZE
stack_bottom:

# --------------------------------------------------------------------------------
# 4. Giriş Noktası (Multiboot Yükleyici Buraya Zıplar)
# --------------------------------------------------------------------------------

.section .text
.global _start
.type _start, @function
_start:
    # rbx: Multiboot Bilgi Yapısı Adresi
    # eax: Multiboot Magic Number (0x2BADB002 veya 0x36D76289 - Multiboot2 için)
    # Multiboot2'de rbx: Multiboot Bilgi Yapısı Adresi, eax: 0

    # Multiboot2 kontrolü: eax'ın 0 olduğundan emin olunur.
    cmp $0, %eax
    jne multiboot_error

    # -----------------------------------
    # 4.1. 32-bit'te Sayfalama Kurulumu
    # -----------------------------------

    # PML4'ü sıfırla (PML4 ve PDPT'nin kendisi Identity Map'ten hariçtir)
    # İlk 1GB (veya ilk 512MB) belleği eşleyeceğiz (Kernel için yeterli).
    
    # PML4'ün ilk girişi PDPT'ye işaret eder (r/w, present)
    movl $pdpt + 0x3, (%ebx)        # rbx'teki Multiboot Adresi yerine temp bir adrese yazılır.
    movl $pdpt + 0x3, pml4          # pml4[0] = &pdpt + P/RW/US bayrakları
    
    # PDPT'nin ilk girişi 1GB'lık dev bir sayfaya (Huge Page) işaret eder (r/w, present, huge)
    # Sayfa Boyutu (PS) bayrağı 1 (0x80) olmalıdır (2MB veya 1GB sayfalar için)
    # 0x0000000000000083 = Present | R/W | Huge Page (1GB)
    movl $0x00000083, pdpt          # pdpt[0] = 0x83 (İlk 1GB Eşlenir)

    # CR4'te PAE (Physical Address Extension) etkinleştir
    movl %cr4, %eax
    orl $0x20, %eax                 # PAE bayrağı (bit 5)
    movl %eax, %cr4

    # CR3'e PML4'ün fiziksel adresini yükle
    movl $pml4, %eax
    movl %eax, %cr3

    # CR0'da Sayfalama (PG) ve Koruma (PE) bayraklarını etkinleştir
    movl %cr0, %eax
    orl $(1<<31) | 0x1, %eax        # PG (bit 31) ve PE (bit 0)
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

    # Artık sayfalama ve 64-bit etkinleştirme bayrakları ayarlandı.
    # 64-bit koda zıplamak için bir segment geçişi yapın (uzun zıplama)
    
    # Long jump to flush the instruction cache and enter 64-bit mode
    # JMP $CODE_SEG, $long_mode_entry (Segment Seçici:Ofset)
    # Basitlik için sadece 64-bit segment seçicisini kullanıyoruz:
    ljmp $CODE_SEG, $long_mode_entry

# --------------------------------------------------------------------------------
# 5. 64-bit Giriş Noktası (Long Mode)
# --------------------------------------------------------------------------------

.code64
.align 8
long_mode_entry:
    # Buradayız: 64-bit Long Mode etkin.
    
    # Veri segmentlerini yeniden yükle (64-bit segment seçici ile)
    # 64-bit modda CS hariç segment yazmaçları yoksayılır, ancak uyumluluk için.
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
    
    # Multiboot Bilgisini (rbx) ve Magic'i (eax) Rust'a geçir.
    # Multiboot2 için rbx = Multiboot Yapı Adresi, eax = 0
    # C ABI'de ilk iki argüman RDI ve RSI'dır.
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
    # Bu noktada, önyükleyici Multiboot2 standardını kullanmamış demektir.
    # Hata durumunu göstermek için basit bir döngü.
error_hang:
    hlt
    jmp error_hang

.size _start, . - _start

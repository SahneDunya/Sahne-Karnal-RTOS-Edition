# src/arch/armv9/boot.s
# ARMv9 (AArch64) Mimarisi için Başlangıç (Boot) Kodu.
# 64-bit EL1/EL2 modunda başlangıç varsayımı.

# --------------------------------------------------------------------------------
# 1. Sabitler ve Adresler
# --------------------------------------------------------------------------------

.section .text
.global _start
.type _start, %function

# Yığın Adresi ve Boyutu (BSS/Data bölümünde tanımlanır)
.equ STACK_SIZE, 0x4000 # 16KB yığın

# --------------------------------------------------------------------------------
# 2. Giriş Noktası (Önyükleyici Buraya Zıplar)
# --------------------------------------------------------------------------------
_start:
    # -----------------------------------
    # 2.1. Çekirdek Çalışma Seviyesi (EL) Kontrolü (Opsiyonel)
    # -----------------------------------
    
    # Mevcut EL'yi oku
    mrs x0, CurrentEL           # CurrentEL'yi x0'a oku
    lsr x0, x0, #2              # x0 = EL (0, 1, 2 veya 3)
    
    # Basit bir çekirdek genellikle EL1'de çalışır. 
    # EL2'de isek, EL1'e düşürmek için yapılandırma gerekir. 
    # Basitlik için sadece EL kontrolü yapılır ve varsayılan EL1'de devam edilir.

    cmp x0, #1
    b.ne  el_error              # EL1 değilse hata (basitlik için)
    
    # -----------------------------------
    # 2.2. GPR'ları Temizle
    # -----------------------------------
    
    # Sıfır yazmacı (xZR/x31) hariç tüm genel amaçlı yazmaçları temizle (x0 - x30)
    mov x1, #0
    mov x2, #0
    mov x3, #0
    mov x4, #0
    mov x5, #0
    mov x6, #0
    mov x7, #0
    mov x8, #0
    mov x9, #0
    mov x10, #0
    mov x11, #0
    mov x12, #0
    mov x13, #0
    mov x14, #0
    mov x15, #0
    mov x16, #0
    mov x17, #0
    mov x18, #0
    mov x19, #0
    mov x20, #0
    mov x21, #0
    mov x22, #0
    mov x23, #0
    mov x24, #0
    mov x25, #0
    mov x26, #0
    mov x27, #0
    mov x28, #0
    mov x29, #0 # FP (Frame Pointer)
    mov x30, #0 # LR (Link Register)
    
    # -----------------------------------
    # 2.3. Yığın Kurulumu
    # -----------------------------------
    
    # Yığın İşaretçisini (SP - x31) ayarla
    # `adrp` talimatı sayfa boyutunda adresleme yapar.
    adrp x1, stack_top          # x1 = stack_top'ın sayfa taban adresi
    add x1, x1, :lo12:stack_top # x1 = stack_top'ın tam adresi
    mov sp, x1                  # SP'ye yığın üstünü yükle

    # -----------------------------------
    # 2.4. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # AArch64 C ABI'de ilk argüman x0'a, ikincisi x1'e geçirilir.
    # U-Boot veya UEFI'den gelen argümanlar (örneğin Device Tree Adresi)
    # genellikle x0'a veya x0/x1'e yüklenir.
    # Basitlik için sadece x0'ı (muhtemelen DTB/ACPI adresi) geçiriyoruz.
    # İlk argüman olarak x0'ı (Boot Bilgisi) kullanıyoruz.
    
    # Rust fonksiyonu `kernel_main(boot_info_addr: u64, reserved: u64)` imzasına sahip olmalıdır.
    # Önyükleyici x0'ı kullandığı için:
    mov x1, #0                  # İkinci argümanı sıfırla (reserved)
    # x0 (Boot Bilgisi) zaten mevcut.
    
    # Rust çekirdeğine kontrolü ver.
    bl kernel_main              # Branch and Link (x30=LR'ye geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    wfi                         # Wait for Interrupt
    b hang                      # Sonsuz döngü

# --------------------------------------------------------------------------------
# 3. Hata Döngüsü
# --------------------------------------------------------------------------------

el_error:
    # EL1'de başlamadıysa hata döngüsü
    wfi_error:
    wfi
    b wfi_error

# --------------------------------------------------------------------------------
# 4. Veri Bölümü
# --------------------------------------------------------------------------------

.section .bss
.align 16
stack_top: 
    .skip STACK_SIZE            # Yığın için yer ayır
stack_bottom:

.size _start, . - _start

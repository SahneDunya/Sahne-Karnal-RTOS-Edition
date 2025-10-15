# src/arch/loongarch64/boot.s
# LoongArch 64 (LA64) Mimarisi için Başlangıç (Boot) Kodu.
# 64-bit modunda başlangıç varsayımı.

# --------------------------------------------------------------------------------
# 1. Sabitler ve Adresler
# --------------------------------------------------------------------------------

.section .text
.global _start
.type _start, @function

# Yığın Adresi ve Boyutu
.equ STACK_SIZE, 0x4000 # 16KB yığın

# --------------------------------------------------------------------------------
# 2. Giriş Noktası (Önyükleyici Buraya Zıplar)
# --------------------------------------------------------------------------------
_start:
    # LoongArch'ta yazmaçlar: $r0=zero, $r1=ra, $r3=sp (Stack Pointer)
    # Argümanlar: $r4 (a0), $r5 (a1), $r6 (a2), ...
    
    # -----------------------------------
    # 2.1. GPR'ları Temizle (Argümanlar hariç)
    # -----------------------------------
    
    # r0 ($zero) her zaman 0'dır, temizlemeye gerek yok.
    # r1 ($ra) temizle.
    li.d $r1, 0
    
    # r2 ($tp), r3 ($sp), r4 ($a0), r5 ($a1) hariç diğerlerini temizle.
    li.d $r2, 0   # $tp (Thread Pointer)
    # $r3 ($sp) yığın kurulurken ayarlanacak.
    # $r4 ve $r5, önyükleyiciden gelen argümanları tutar.
    
    # Diğer genel amaçlı yazmaçları sıfırla (r6 - r31)
    li.d $r6, 0
    li.d $r7, 0
    li.d $r8, 0
    li.d $r9, 0
    li.d $r10, 0
    li.d $r11, 0
    li.d $r12, 0
    li.d $r13, 0
    li.d $r14, 0
    li.d $r15, 0
    li.d $r16, 0
    li.d $r17, 0
    li.d $r18, 0
    li.d $r19, 0
    li.d $r20, 0
    li.d $r21, 0
    li.d $r22, 0
    li.d $r23, 0
    li.d $r24, 0
    li.d $r25, 0
    li.d $r26, 0
    li.d $r27, 0
    li.d $r28, 0
    li.d $r29, 0
    li.d $r30, 0
    li.d $r31, 0

    # -----------------------------------
    # 2.2. Yığın Kurulumu
    # -----------------------------------
    
    # Yığın İşaretçisini ($sp - r3) ayarla.
    # LoongArch'ta $sp'nin 16-bayt hizalı olması gerekir.
    
    # 'la.abs' ile yığın üstünün mutlak adresini r3'e yükle.
    la.abs $r3, stack_top
    
    # -----------------------------------
    # 2.3. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # LoongArch C ABI'sinde ilk argümanlar $r4 (a0) ve $r5 (a1)'e geçirilir.
    # Önyükleyici (örneğin U-Boot) genellikle:
    # $r4 (a0): Device Tree Blob (DTB) veya Boot Bilgisi adresi
    # $r5 (a1): Reserved veya 0
    
    # Bu varsayımla, argümanlar zaten doğru yazmaçlardadır ($r4, $r5).
    
    # Rust çekirdeğine kontrolü ver.
    bl kernel_main          # Branch and Link (r1/$ra'ya geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    # PLV0 (Machine/Kernel Mode) için `idle` veya `dbar` gibi talimatlar kullanılabilir.
    idle                    # Wait for Interrupt/Event
    b hang                  # Sonsuz döngü

# --------------------------------------------------------------------------------
# 3. Hata Döngüsü
# --------------------------------------------------------------------------------

.global error_hang
error_hang:
    # Hata durumunda (örneğin bootloader uyumsuzluğu)
    idle
    b error_hang

# --------------------------------------------------------------------------------
# 4. Veri Bölümü
# --------------------------------------------------------------------------------

.section .bss
.align 16
stack_top: 
    .skip STACK_SIZE            # Yığın için yer ayır
stack_bottom:

.size _start, . - _start

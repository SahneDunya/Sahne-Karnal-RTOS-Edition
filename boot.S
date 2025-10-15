# src/arch/openrisc64/boot.s
# OpenRISC 64 (OR64) Mimarisi için Başlangıç (Boot) Kodu.
# 64-bit modunda başlangıç varsayımı.

# --------------------------------------------------------------------------------
# 1. Sabitler ve Adresler
# --------------------------------------------------------------------------------

.section .text
.global _start
.type _start, @function

# Yığın Adresi ve Boyutu
.equ STACK_SIZE, 0x4000      # 16KB yığın

# --------------------------------------------------------------------------------
# 2. Giriş Noktası (Önyükleyici Buraya Zıplar)
# --------------------------------------------------------------------------------
_start:
    # Önyükleyici (Firmware/U-Boot) genellikle:
    # r3: Device Tree Blob (DTB) veya Boot Bilgisi adresi
    # r4: İkinci Argüman
    
    # Argümanları geçici olarak s9 ve s10'a kaydet (r3, r4)
    # OpenRISC'te r9-r18 's' yazmaçları olarak kabul edilebilir.
    # r3 -> r9
    # r4 -> r10
    
    l.ori r9, r0, 0             # r9 = r3 (l.mflr r9, r3 talimatı OpenRISC'te yok)
    l.ori r10, r4, 0            # r10 = r4 (Argüman kopyalamak için l.ori kullanılır)

    # -----------------------------------
    # 2.1. Yazmaçları Temizleme
    # -----------------------------------
    
    # r1 (SP), r2 (LR), r9 (Arg1), r10 (Arg2) hariç diğerlerini temizle.
    # OpenRISC'te l.ori rD, rS, 0x0 ile rD sıfırlanır (r0'ın sıfır olduğunu kullanarak)
    
    l.ori r3, r0, 0
    l.ori r4, r0, 0
    l.ori r5, r0, 0
    l.ori r6, r0, 0
    l.ori r7, r0, 0
    l.ori r8, r0, 0
    
    # r11'den r31'e kadar temizle
    l.ori r11, r0, 0
    l.ori r12, r0, 0
    l.ori r13, r0, 0
    l.ori r14, r0, 0
    l.ori r15, r0, 0
    l.ori r16, r0, 0
    l.ori r17, r0, 0
    l.ori r18, r0, 0
    l.ori r19, r0, 0
    l.ori r20, r0, 0
    l.ori r21, r0, 0
    l.ori r22, r0, 0
    l.ori r23, r0, 0
    l.ori r24, r0, 0
    l.ori r25, r0, 0
    l.ori r26, r0, 0
    l.ori r27, r0, 0
    l.ori r28, r0, 0
    l.ori r29, r0, 0
    l.ori r30, r0, 0
    l.ori r31, r0, 0
    
    # -----------------------------------
    # 2.2. Yığın Kurulumu
    # -----------------------------------
    
    # Yığın İşaretçisini (r1) ayarla.
    # l.mova talimatı ile yığın üstünün mutlak adresini r1'e yükle.
    l.mova r1, stack_top
    
    # -----------------------------------
    # 2.3. Boot Argümanlarını Yükle
    # -----------------------------------
    
    # Kaydettiğimiz argümanları Rust ABI'sine uygun olarak r3 ve r4'e yükle
    l.ori r3, r9, 0             # Argüman 1 (Boot Bilgisi)
    l.ori r4, r10, 0            # Argüman 2
    
    # -----------------------------------
    # 2.4. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Rust çekirdeğine kontrolü ver.
    l.jal kernel_main           # Jump and Link (r2/LR'ye geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    # Basit bir döngü
    l.nop                       # No Operation
    l.j hang                    # Sonsuz döngü

# --------------------------------------------------------------------------------
# 3. Hata Döngüsü
# --------------------------------------------------------------------------------

.global error_hang
error_hang:
    # Hata durumunda
    l.nop
    l.j error_hang

# --------------------------------------------------------------------------------
# 4. Veri Bölümü
# --------------------------------------------------------------------------------

.section .bss
.align 16
stack_top: 
    .skip STACK_SIZE            # Yığın için yer ayır
stack_bottom:

.size _start, . - _start
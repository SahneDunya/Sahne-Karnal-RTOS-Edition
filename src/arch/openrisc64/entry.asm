# src/arch/openrisc64/entry.s
# OpenRISC 64 (OR64) Mimarisi için Çekirdek Giriş Noktası Kodu.
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
    # Argümanlar: r3, r4, r5, ... (OpenRISC ABI kuralına göre)
    
    # Argümanları geçici olarak kalıcı yazmaçlara (r9, r10, r11) kaydet.
    # r3 -> r9
    # r4 -> r10
    # r5 -> r11
    
    l.ori r9, r3, 0             # Argüman 1'i r9'a kopyala
    l.ori r10, r4, 0            # Argüman 2'yi r10'a kopyala
    l.ori r11, r5, 0            # Argüman 3'ü r11'e kopyala

    # -----------------------------------
    # 2.1. Yazmaçları Temizleme
    # -----------------------------------
    
    # r9, r10, r11 (Argümanlar) ve r1 (SP) hariç diğerlerini sıfırla.
    # r0 daima 0'dır.
    
    l.ori r2, r0, 0             # r2 (LR)
    
    # r3'ten r8'e kadar sıfırla (argümanlar)
    l.ori r3, r0, 0
    l.ori r4, r0, 0
    l.ori r5, r0, 0
    l.ori r6, r0, 0
    l.ori r7, r0, 0
    l.ori r8, r0, 0
    
    # r12'den r31'e kadar sıfırla (r9, r10, r11 hariç)
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
    l.mova r1, stack_top
    
    # -----------------------------------
    # 2.3. Boot Argümanlarını Yükle
    # -----------------------------------
    
    # Kaydettiğimiz argümanları Rust ABI'sine uygun olarak r3, r4, r5'e yükle
    l.ori r3, r9, 0             # Argüman 1 (Boot Bilgisi)
    l.ori r4, r10, 0            # Argüman 2
    l.ori r5, r11, 0            # Argüman 3
    
    # -----------------------------------
    # 2.4. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Rust çekirdeğine kontrolü ver.
    l.jal kernel_main           # Jump and Link (r2/LR'ye geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
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
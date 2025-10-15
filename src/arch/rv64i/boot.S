# src/arch/rv64i/boot.s
# RISC-V 64 (RV64I) Mimarisi için Başlangıç (Boot) Kodu.
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
    # Önyükleyici (Firmware/SBI/U-Boot) genellikle:
    # a0 ($x10): Argüman 1 (Genellikle Device Tree Blob adresi)
    # a1 ($x11): Argüman 2 (Genellikle Hart ID - Çekirdek Numarası)
    
    # Argümanları geçici olarak kalıcı yazmaçlara (s0, s1) kaydet.
    # a0 ($x10) -> s0 ($x8)
    # a1 ($x11) -> s1 ($x9)
    
    mv s0, a0                   # Argüman 1'i s0'a kopyala
    mv s1, a1                   # Argüman 2'yi s1'e kopyala

    # -----------------------------------
    # 2.1. Yazmaçları Temizleme
    # -----------------------------------
    
    # s0, s1 (Argümanlar) ve s2-s11 (diğer kalıcı yazmaçlar) hariç diğerlerini sıfırla.
    # $x0 (zero) her zaman 0'dır.
    
    li ra, 0                    # $x1 (ra)
    # sp ($x2) yığın kurulurken ayarlanacak.
    li gp, 0                    # $x3 (gp)
    li tp, 0                    # $x4 (tp)
    li t0, 0                    # $x5 (t0)
    li t1, 0                    # $x6 (t1)
    li t2, 0                    # $x7 (t2)
    
    # Argümanlar (a0, a1) temizlenmedi (s0, s1'de saklandı)
    li a2, 0                    # $x12
    li a3, 0                    # $x13
    li a4, 0                    # $x14
    li a5, 0                    # $x15
    li a6, 0                    # $x16
    li a7, 0                    # $x17
    
    # Kalan kalıcı yazmaçları sıfırla
    li s2, 0                    # $x18
    li s3, 0                    # $x19
    li s4, 0                    # $x20
    li s5, 0                    # $x21
    li s6, 0                    # $x22
    li s7, 0                    # $x23
    li s8, 0                    # $x24 (fp)
    li s9, 0                    # $x25
    li s10, 0                   # $x26
    li s11, 0                   # $x27
    
    li t3, 0                    # $x28
    li t4, 0                    # $x29
    li t5, 0                    # $x30
    li t6, 0                    # $x31
    
    # -----------------------------------
    # 2.2. Yığın Kurulumu
    # -----------------------------------
    
    # Yığın İşaretçisini ($sp - $x2) ayarla.
    # 'la' (Load Address) ile yığın üstünün adresini $sp'ye yükle.
    
    la sp, stack_top            # sp'ye yığın üstünü yükle

    # -----------------------------------
    # 2.3. Boot Argümanlarını Yükle
    # -----------------------------------
    
    # Kaydettiğimiz argümanları Rust ABI'sine uygun olarak a0 ve a1'e yükle
    mv a0, s0                   # Argüman 1 (Boot Bilgisi/DTB Adresi)
    mv a1, s1                   # Argüman 2 (Hart ID)
    
    # -----------------------------------
    # 2.4. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Rust çekirdeğine kontrolü ver.
    call kernel_main            # Call/Jump and Link (ra/$x1'e geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    # İşlemciyi düşük güç/kesme bekleme moduna sok.
    wfi                         # Wait For Interrupt
    j hang                      # Sonsuz döngü

# --------------------------------------------------------------------------------
# 3. Hata Döngüsü
# --------------------------------------------------------------------------------

.global error_hang
error_hang:
    # Hata durumunda
    wfi
    j error_hang

# --------------------------------------------------------------------------------
# 4. Veri Bölümü
# --------------------------------------------------------------------------------

.section .bss
.align 16
stack_top: 
    .skip STACK_SIZE            # Yığın için yer ayır
stack_bottom:

.size _start, . - _start

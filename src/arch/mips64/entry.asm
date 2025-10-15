# src/arch/mips64/entry.s
# MIPS 64 (MIPS64) Mimarisi için Çekirdek Giriş Noktası Kodu.
# 64-bit Kernel Mode (KSEG0/KSEG1) modunda başlangıç varsayımı.

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
    # Argümanlar: $a0 ($4), $a1 ($5), $a2 ($6), $a3 ($7)
    
    # Argümanları geçici olarak kalıcı yazmaçlara ($s0, $s1, $s2) kaydet.
    # $a0 ($4) -> $s0 ($16)
    # $a1 ($5) -> $s1 ($17)
    # $a2 ($6) -> $s2 ($18)
    
    move $s0, $a0               # Argüman 1'i $s0'a kopyala
    move $s1, $a1               # Argüman 2'yi $s1'e kopyala
    move $s2, $a2               # Argüman 3'ü $s2'ye kopyala
    
    # -----------------------------------
    # 2.1. Yazmaçları Temizleme
    # -----------------------------------
    
    # $s0, $s1, $s2 (boot argümanları) hariç diğerlerini sıfırla
    # Argüman yazmaçları ($a0-$a3) temizlenecek ve daha sonra geri yüklenecek.
    
    li $v0, 0                   # $v0 ($2)
    li $v1, 0                   # $v1 ($3)
    
    li $a0, 0
    li $a1, 0
    li $a2, 0
    li $a3, 0
    
    li $t0, 0                   # $t0 ($8)
    li $t1, 0
    li $t2, 0
    li $t3, 0
    li $t4, 0
    li $t5, 0
    li $t6, 0
    li $t7, 0                   # $t7 ($15)
    
    # $s3'ten $s7'ye kadar sıfırla
    li $s3, 0                   # $s3 ($19)
    li $s4, 0
    li $s5, 0
    li $s6, 0
    li $s7, 0                   # $s7 ($23)
    
    li $t8, 0                   # $t8 ($24)
    li $t9, 0                   # $t9 ($25)
    
    li $gp, 0                   # $gp ($28)
    li $fp, 0                   # $fp ($30)
    li $ra, 0                   # $ra ($31)

    # -----------------------------------
    # 2.2. Yığın Kurulumu
    # -----------------------------------
    
    # Yığın İşaretçisini ($sp - $29) ayarla.
    la $at, stack_top           # $at ($1) ile yığın üstünün adresini al
    move $sp, $at               # $sp'ye yığın üstünü yükle

    # -----------------------------------
    # 2.3. Boot Argümanlarını Yükle
    # -----------------------------------
    
    # Kaydettiğimiz argümanları Rust ABI'sine uygun olarak $a0, $a1, $a2'ye yükle
    move $a0, $s0               # Argüman 1 (Boot Bilgisi)
    move $a1, $s1               # Argüman 2
    move $a2, $s2               # Argüman 3
    
    # -----------------------------------
    # 2.4. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Rust çekirdeğine kontrolü ver.
    jal kernel_main             # Jump and Link ($ra/$31'e geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    wait                        # Wait for Interrupt
    b hang                      # Sonsuz döngü

# --------------------------------------------------------------------------------
# 3. Hata Döngüsü
# --------------------------------------------------------------------------------

.global error_hang
error_hang:
    # Hata durumunda
    wait
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
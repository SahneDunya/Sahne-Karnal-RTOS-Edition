# src/arch/mips64/boot.s
# MIPS 64 (MIPS64) Mimarisi için Başlangıç (Boot) Kodu.
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
    # Önyükleyici (U-Boot/Firmware) genellikle:
    # $a0 ($4): Device Tree Blob (DTB) veya Boot Bilgisi adresi
    # $a1 ($5): Board ID veya İkinci Argüman
    # $a2 ($6): Üçüncü Argüman
    # $a3 ($7): Dördüncü Argüman
    
    # Argümanları geçici olarak s1'den s3'e kaydet (a0, a1, a2)
    # $a0 ($4) -> $s0 ($16)
    # $a1 ($5) -> $s1 ($17)
    # $a2 ($6) -> $s2 ($18)
    
    move $s0, $a0
    move $s1, $a1
    move $s2, $a2
    
    # -----------------------------------
    # 2.1. Yığın Kurulumu
    # -----------------------------------
    
    # Yığın İşaretçisini ($sp - $29) ayarla.
    # %gp (Global Pointer) MIPS'te önemlidir, ancak bu minimal boot kodunda atlanır.
    
    # 'la' (Load Address) ile yığın üstünün mutlak adresini at ($1) yazmacına yükle.
    la $at, stack_top
    move $sp, $at               # $sp'ye yığın üstünü yükle

    # -----------------------------------
    # 2.2. GPR'ları Temizle
    # -----------------------------------
    
    # $s0, $s1, $s2 (boot argümanları) hariç diğerlerini temizle
    # r1, r2, r3, r8-r15 (t0-t7), r24, r25 (t8, t9), r28 (gp), r30 (fp), r31 (ra)
    
    li $v0, 0
    li $v1, 0
    li $t0, 0
    li $t1, 0
    li $t2, 0
    li $t3, 0
    li $t4, 0
    li $t5, 0
    li $t6, 0
    li $t7, 0
    li $t8, 0
    li $t9, 0
    li $gp, 0
    li $fp, 0
    li $ra, 0

    # -----------------------------------
    # 2.3. Boot Argümanlarını Yükle
    # -----------------------------------
    
    # Kaydettiğimiz argümanları Rust ABI'sine uygun olarak $a0 ve $a1'e yükle
    # $a0, $a1, $a2, $a3 Rust'ın ilk dört argümanıdır.
    move $a0, $s0               # Argüman 1 (Boot Bilgisi)
    move $a1, $s1               # Argüman 2
    move $a2, $s2               # Argüman 3 (a3 boş bırakılabilir)
    
    # -----------------------------------
    # 2.4. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Rust çekirdeğine kontrolü ver.
    jal kernel_main             # Jump and Link (ra/$31'e geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    # Kesmeleri kapat
    # cli                     # MIPS'te kesmeleri kapatmak için CP0 Status Register kullanılır
    
    # Basit bir döngü
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
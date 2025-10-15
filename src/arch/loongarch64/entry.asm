# src/arch/loongarch64/entry.s
# LoongArch 64 (LA64) Mimarisi için Çekirdek Giriş Noktası Kodu.
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
    # Argümanlar: $r4 (a0), $r5 (a1)
    
    # Argümanları geçici olarak kalıcı yazmaçlara ($s0, $s1) kaydet.
    # $r4 ($a0) -> $r8 ($s0)
    # $r5 ($a1) -> $r9 ($s1)
    
    ori $r8, $r4, 0             # Argüman 1'i $s0'a kopyala
    ori $r9, $r5, 0             # Argüman 2'yi $s1'e kopyala

    # -----------------------------------
    # 2.1. GPR'ları Temizle
    # -----------------------------------
    
    # $r4 ve $r5 (Argümanlar) temizlenecek çünkü değerleri $r8, $r9'da saklanıyor.
    # $r0 ($zero) her zaman 0'dır. $r8 ($s0), $r9 ($s1) hariç diğerlerini temizle.
    
    li.d $r1, 0                 # $r1 (ra)
    li.d $r2, 0                 # $r2 (tp)
    # $r3 (sp) yığın kurulurken ayarlanacak.
    
    li.d $r4, 0                 # $r4 (a0)
    li.d $r5, 0                 # $r5 (a1)
    li.d $r6, 0                 # $r6 (a2)
    li.d $r7, 0                 # $r7 (a3)
    
    # r10'dan r31'e kadar sıfırla (r8, r9 hariç)
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
    
    # Yığın İşaretçisini ($sp - $r3) ayarla.
    # 'la.abs' ile yığın üstünün mutlak adresini $r3'e yükle.
    la.abs $r3, stack_top
    
    # -----------------------------------
    # 2.3. Boot Argümanlarını Yükle
    # -----------------------------------
    
    # Kaydettiğimiz argümanları Rust ABI'sine uygun olarak $r4 (a0) ve $r5 (a1)'e yükle
    ori $r4, $r8, 0             # Argüman 1 ($s0 -> $a0)
    ori $r5, $r9, 0             # Argüman 2 ($s1 -> $a1)
    
    # -----------------------------------
    # 2.4. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Rust çekirdeğine kontrolü ver.
    bl kernel_main          # Branch and Link ($r1/$ra'ya geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    idle                    # Wait for Interrupt/Event
    b hang                  # Sonsuz döngü

# --------------------------------------------------------------------------------
# 3. Hata Döngüsü
# --------------------------------------------------------------------------------

.global error_hang
error_hang:
    # Hata durumunda
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
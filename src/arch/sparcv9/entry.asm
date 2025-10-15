# src/arch/sparcv9/entry.s
# SPARC V9 (UltraSPARC) Mimarisi için Çekirdek Giriş Noktası Kodu.
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
    # SPARC V9 ABI'ye göre argümanlar %o0, %o1, %o2, ... yazmaçlarındadır.
    # Bunlar 'call kernel_main' ile otomatik olarak %i0, %i1, %i2'ye kaydırılacaktır.
    
    # -----------------------------------
    # 2.1. Global Yazmaçları Temizleme
    # -----------------------------------
    
    # %g0 daima 0'dır. Diğerlerini sıfırla.
    # Argümanlar (%o0-o7) korunuyor ve bir sonraki pencereye aktarılacak.
    
    mov %g0, %g1
    mov %g0, %g2
    mov %g0, %g3
    mov %g0, %g4
    mov %g0, %g5
    mov %g0, %g6
    mov %g0, %g7
    
    # -----------------------------------
    # 2.2. Yığın Kurulumu
    # -----------------------------------
    
    # Yığın İşaretçisini (%o6 - %sp) ayarla.
    
    # Yığın üstünün mutlak 64-bit adresini %o6'ya yükle.
    # Bu, derleyicinin 64-bit adres yükleme makrolarına güvenir.
    sethi %hix(stack_top), %o6
    or %o6, %lox(stack_top), %o6
    sethi %him(stack_top), %g1
    or %o6, %g1, %o6
    sllx %o6, 32, %o6
    sethi %hi(stack_top), %g1
    or %g1, %lo(stack_top), %g1
    or %o6, %g1, %o6
    
    # Not: %o6 (%sp)'nin 8 bayt eksiği, ilk yığın çerçevesinin başlangıcıdır (kaydedilen %i0-i7 için yer).

    # -----------------------------------
    # 2.3. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Rust çekirdeğine kontrolü ver.
    # 'call' pencereyi kaydırır: %o0-o7 -> %i0-i7 (Argümanlar korunur)
    call kernel_main, 0         
    nop                         # Gecikme Yuvası (Delay Slot)
    
    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    nop                         # No Operation
    ba hang                     # Sonsuz döngü

# --------------------------------------------------------------------------------
# 3. Hata Döngüsü
# --------------------------------------------------------------------------------

.global error_hang
error_hang:
    # Hata durumunda
    nop
    ba error_hang

# --------------------------------------------------------------------------------
# 4. Veri Bölümü
# --------------------------------------------------------------------------------

.section .bss
.align 16
stack_top: 
    .skip STACK_SIZE            # Yığın için yer ayır
stack_bottom:

.size _start, . - _start
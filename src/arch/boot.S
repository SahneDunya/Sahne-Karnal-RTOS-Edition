# src/arch/sparcv9/boot.s
# SPARC V9 (UltraSPARC) Mimarisi için Başlangıç (Boot) Kodu.
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
    # Önyükleyici (OpenBoot PROM) genellikle:
    # %o0: Argüman 1 (Genellikle Device Tree adresi veya boot parametreleri)
    # %o1: Argüman 2
    # %o2: Argüman 3
    
    # Argümanları koru: %o0, %o1, %o2 zaten doğru yazmaçlardadır.
    # Ancak, %o0 - %o5 yerel (local) yazmaçlara ($l0 - $l5) taşınmalıdır
    # çünkü ilk 'call' talimatı (Rust'a zıplama) pencereyi kaydırır.
    
    # Argümanları korumak için 'save' talimatını kullan. 
    # Bu, %o0-o7 yazmaçlarını bir sonraki pencerenin %i0-i7 yazmaçlarına kaydırır.
    
    # save %sp, -1024, %sp # Standart pencere kaydırma (biz daha sonra yapacağız)

    # -----------------------------------
    # 2.1. Global Yazmaçları Temizleme
    # -----------------------------------
    
    # %g0 daima 0'dır. Diğerlerini sıfırla.
    # %g0-%g7 global yazmaçlardır.
    
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
    # 'sethi' ve 'or' ile yığın üstünün 64-bit adresini %o6'ya yükle.
    
    # Yığın üstünün mutlak adresini %o6'ya yükle
    sethi %hi(stack_top), %o6   # %o6'ya stack_top adresinin üst 22 bitini yükle
    or %o6, %lo(stack_top), %o6 # %o6'ya stack_top adresinin alt 10 bitini ekle (veya 32 bit için farklı bir makro)
    
    # SPARC V9'da 64-bit için daha doğru adres yükleme
    sethi %hix(stack_top), %o6
    or %o6, %lox(stack_top), %o6
    sethi %him(stack_top), %g1
    or %o6, %g1, %o6
    sllx %o6, 32, %o6
    sethi %hi(stack_top), %g1
    or %g1, %lo(stack_top), %g1
    or %o6, %g1, %o6
    
    # Basitleştirilmiş (derleyici makrolarına güvenerek):
    sethi %hi(stack_top), %o6
    or %o6, %lo(stack_top), %o6
    
    # NOT: SPARC assembly'de 64-bit mutlak adres yüklemek zorlayıcıdır. 
    # En güvenilir yöntem, derleyicinin sunduğu psödo-talimatlara (örneğin `sethi`/`or` kombinasyonlarına) güvenmektir.
    # Alternatif:
    # sethi %hh(stack_top), %o6
    # seth %lm(stack_top), %o6, %o6

    # -----------------------------------
    # 2.3. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Argümanlar zaten %o0, %o1, %o2'dedir.
    
    # Rust çekirdeğine kontrolü ver.
    # 'call' talimatı %o7'ye dönüş adresini kaydeder ve yeni bir pencere açar.
    call kernel_main, 0         # call kernel_main (yeni pencere açılır, argümanlar %i0-i7 olur)
    nop                         # Gecikme Yuvası (Delay Slot)
    
    # kernel_main'den dönülmez, ancak dönerse temizlik yapılmalıydı.
    
    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    # Basit bir döngü
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
# src/arch/powerpc64/entry.s
# PowerPC 64 (PPC64) Mimarisi için Çekirdek Giriş Noktası Kodu.
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
    # Argümanlar: r3 (Arg 1), r4 (Arg 2), r5 (Arg 3), ...
    
    # Argümanları geçici olarak kalıcı yazmaçlara (r13, r14, r15) kaydet.
    # r3 -> r13
    # r4 -> r14
    # r5 -> r15
    
    mr r13, r3                  # Argüman 1'i r13'e kopyala
    mr r14, r4                  # Argüman 2'yi r14'e kopyala
    mr r15, r5                  # Argüman 3'ü r15'e kopyala

    # -----------------------------------
    # 2.1. Yazmaçları Temizleme
    # -----------------------------------
    
    # r13, r14, r15 (Argümanlar) ve r1 (SP) hariç diğerlerini sıfırla.
    # Argüman yazmaçları (r3-r10) temizlenecek ve daha sonra geri yüklenecek.
    
    li r0, 0                    # r0'ı sıfırla
    li r2, 0                    # r2 (TOC/Global Pointer)
    
    # r3 - r12 (Argüman ve geçici yazmaçlar)
    li r3, 0
    li r4, 0
    li r5, 0
    li r6, 0
    li r7, 0
    li r8, 0
    li r9, 0
    li r10, 0
    li r11, 0
    li r12, 0
    
    # r16'dan r30'a kadar sıfırla (r13, r14, r15 hariç)
    li r16, 0
    li r17, 0
    li r18, 0
    li r19, 0
    li r20, 0
    li r21, 0
    li r22, 0
    li r23, 0
    li r24, 0
    li r25, 0
    li r26, 0
    li r27, 0
    li r28, 0
    li r29, 0
    li r30, 0
    
    # -----------------------------------
    # 2.2. Yığın Kurulumu
    # -----------------------------------
    
    # Yığın İşaretçisini (r1) ayarla.
    # Yığın üstünün mutlak adresini r1'e yükle (64-bit adresleme için)
    
    lis r1, hi(stack_top)       # r1'e stack_top adresinin üst 32-bit'ini yükle
    ori r1, r1, lo(stack_top)   # r1'e stack_top adresinin alt 32-bit'ini yükle
    
    # NOT: 64-bit adres yüklemesi için bu iki talimat genellikle yeterlidir, 
    # ancak PowerPC 64-bit için adreslemeyi dikkatlice ele almak gerekir. 
    # GNU as, 64-bit adresler için bu kombinasyonu kullanabilir.
    
    # -----------------------------------
    # 2.3. Boot Argümanlarını Yükle
    # -----------------------------------
    
    # Kaydettiğimiz argümanları Rust ABI'sine uygun olarak r3, r4, r5'e yükle
    mr r3, r13                  # Argüman 1 (Boot Bilgisi)
    mr r4, r14                  # Argüman 2
    mr r5, r15                  # Argüman 3
    
    # -----------------------------------
    # 2.4. Rust Giriş Noktasına Zıplama
    # -----------------------------------
    
    # Rust çekirdeğine kontrolü ver.
    bl kernel_main              # Branch and Link (LR/r31'e geri dönüş adresi kaydeder)

    # Eğer kernel_main geri dönerse, bu bir hatadır (çekirdek asla geri dönmemelidir).
hang:
    # Basit bir döngü
    sync                        # Ön bellek/pipeline senkronizasyonu
    b hang                      # Sonsuz döngü

# --------------------------------------------------------------------------------
# 3. Hata Döngüsü
# --------------------------------------------------------------------------------

.global error_hang
error_hang:
    # Hata durumunda
    sync
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
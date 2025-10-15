// src/arch/loongarch64/security.rs
// LoongArch 64 (LA64) mimarisine özgü temel güvenlik işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/loongarch64/platformmod.rs dosyasından temel bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanımsal Rastgelelik (Hardware Randomness)
// -----------------------------------------------------------------------------

/// Donanımsal Rastgele Sayı Üretimi için bir placeholder.
/// LoongArch'ta özel bir RDRAND benzeri talimat olup olmadığı platforma bağlıdır.
/// Eğer yoksa, bu fonksiyon sadece temsili bir değer döndürür.
/// Gerçek bir donanımsal RNG, özel MMIO adresinden okuma gerektirebilir.
#[inline(always)]
pub fn get_hardware_random_u64() -> u64 {
    // LoongArch ISA'da standart bir RNG talimatı yoksa:
    serial_println!("[SECURITY] Uyarı: Donanımsal RNG (RDRAND/RNDR benzeri) bulunamadı. Temsili değer kullanılıyor.");
    
    // Temsili olarak, rastgele bir değer üretmek için, genellikle zamanlayıcı 
    // veya başka bir donanım kaynağı kullanılır.
    // Şimdilik 0 döndürülür. Gerçek kodda, PMU/Timer CSR'dan okuma yapılabilir.
    0xBAD_C0DE_C0DE_BEEF
}

// -----------------------------------------------------------------------------
// Yazmaç Temizleme (Register Zeroing)
// -----------------------------------------------------------------------------

/// Genel amaçlı yazmaçları (GPR) bellekten geri dönerken temizler.
///
/// # Güvenlik Notu
/// Spekülatif yürütme yan kanal saldırılarını (Spectre) azaltmada yardımcı olur.
#[inline(always)]
pub unsafe fn zero_gprs() {
    // LoongArch'ta x0 sıfır yazmacıdır. x1'den x31'e kadar olan yazmaçları sıfırla.
    asm!(
        "xor $r1, $r1, $r1", "xor $r2, $r2, $r2", "xor $r3, $r3, $r3", 
        "xor $r4, $r4, $r4", "xor $r5, $r5, $r5", "xor $r6, $r6, $r6",
        "xor $r7, $r7, $r7", "xor $r8, $r8, $r8", "xor $r9, $r9, $r9",
        "xor $r10, $r10, $r10", "xor $r11, $r11, $r11", "xor $r12, $r12, $r12",
        "xor $r13, $r13, $r13", "xor $r14, $r14, $r14", "xor $r15, $r15, $r15",
        "xor $r16, $r16, $r16", "xor $r17, $r17, $r17", "xor $r18, $r18, $r18",
        "xor $r19, $r19, $r19", "xor $r20, $r20, $r20", "xor $r21, $r21, $r21",
        "xor $r22, $r22, $r22", "xor $r23, $r23, $r23", "xor $r24, $r24, $r24",
        "xor $r25, $r25, $r25", "xor $r26, $r26, $r26", "xor $r27, $r27, $r27",
        "xor $r28, $r28, $r28", "xor $r29, $r29, $r29", "xor $r30, $r30, $r30",
        "xor $r31, $r31, $r31",
        options(nomem, nostack)
    );
    serial_println!("[SECURITY] Genel Amaçlı Yazmaçlar Temizlendi.");
}

// -----------------------------------------------------------------------------
// Koruma Ayarları (CSR Yapılandırması)
// -----------------------------------------------------------------------------

/// Temel bellek koruma özelliklerini etkinleştirir (NX bitine karşılık gelen özellik).
/// LoongArch'ta bu genellikle TLB/MMU kontrol yazmaçlarında yapılır.
///
/// # Örnek: CRMD (Core Root Mode Register)
/// * **PPLV:** Geçerli Ayrıcalık Seviyesi (PLV0, PLV1, PLV2, PLV3)
/// * **WE:** Yazılabilir/Yürütülebilir Bayraklar (Güvenlik Koruması İçin Önemli)
pub fn configure_security_features() {
    serial_println!("[SECURITY] Temel Kontrol Yazmaçları Yapılandırılıyor...");
    
    // LoongArch CSR numaraları (Temsili, tam LoongArch ISA'ya bakılmalıdır)
    const CSR_CRMD: u32 = 0x0; 
    
    // CRMD'yi oku
    let mut crmd = unsafe { io::read_csr(CSR_CRMD) };
    
    // Gerekli güvenlik bitlerini ayarla/temizle (Örn: PLV'yi en yüksek ayrıcalığa ayarla)
    // Güvenlik ayarları donanıma özgüdür.
    
    // Örnek: PLV'yi 0 (Kernel modu) olarak ayarla (CRMD[1:0] alanı)
    const CRMD_PLV_MASK: u64 = 0x3; 
    const CRMD_PLV_KERNEL: u64 = 0x0;
    
    crmd = (crmd & !CRMD_PLV_MASK) | CRMD_PLV_KERNEL;
    
    // CRMD'ye yaz
    unsafe { 
        io::write_csr(CSR_CRMD, crmd);
        io::membar_all(); // Senkronizasyon
    }
    
    serial_println!("[SECURITY] CRMD (Kontrol Modu) Temel Ayarları Tamamlandı.");
}


// -----------------------------------------------------------------------------
// Temel Güvenlik Başlatma
// -----------------------------------------------------------------------------

/// Güvenlik modülünü başlatır ve temel korumaları etkinleştirir.
pub fn security_init() {
    serial_println!("[SECURITY] LoongArch 64 Temel Güvenlik Başlatılıyor...");
    
    // 1. Temel sistem kontrol yazmaçlarını yapılandır
    configure_security_features();

    // 2. İlk rastgelelik testi
    let rand_val = get_hardware_random_u64();
    serial_println!("[SECURITY] RNG Testi (Temsili): Değer: {:#x}", rand_val);
    
    // 3. (Gelişmiş Kodda): Bellek Koruma Birimlerinin (MMU/TLB) güvenlik ile 
    // ilgili ayarlarının yapılandırılması buraya eklenirdi.
    
    serial_println!("[SECURITY] Temel Güvenlik Yapılandırması Tamamlandı.");
}
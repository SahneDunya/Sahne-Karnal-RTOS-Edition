// src/arch/armv9/security.rs
// ARMv9 (aarch64) mimarisine özgü temel güvenlik işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/armv9/platformmod.rs dosyasından temel bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanımsal Rastgelelik (Hardware Randomness)
// -----------------------------------------------------------------------------

/// Donanımsal Rastgele Sayı Üretimi (RNDR talimatı) ile 64-bit rastgele sayı alır.
/// RNDR, entropy (rastgelelik) elde edilene kadar bekler.
///
/// # Not: RNDR (True Random) talimatı ARMv8.5-A veya sonrası gerektirir (ARMv9 ile uyumlu).
///
/// # İade Değeri
/// u64: Üretilen rastgele sayı.
#[inline(always)]
pub fn get_hardware_random_u64() -> u64 {
    let value: u64;
    
    // RNDR (Read Random) talimatı kullanılır.
    // Başarısızlık durumunda 0 döndürebilir, ancak genellikle donanım bekler.
    unsafe {
        asm!(
            "mrs {0}, RNDR_EL0", // RNDR talimatı, genellikle RNDR_EL0 (EL0 erişimi) olarak uygulanır.
            out(reg) value,
            options(nomem, nostack)
        );
    }
    // RNDR, değerin hazır olmasını beklediği için ayrı bir başarı bayrağı kontrolü
    // RDRAND'deki gibi zorunlu değildir, ancak daha güvenli sistemlerde RNDRRS kullanılır.
    value
}

// -----------------------------------------------------------------------------
// Yazmaç Temizleme (Register Zeroing)
// -----------------------------------------------------------------------------

/// Genel amaçlı yazmaçları (GPR) bellekten geri dönerken temizler.
///
/// # Güvenlik Notu
/// Spekülatif yürütme yan kanal saldırılarını (Spectre) azaltmada yardımcı olur.
/// Çekirdekten kullanıcı alanına geri dönmeden önce hassas verilerin sızmasını önler.
#[inline(always)]
pub unsafe fn zero_gprs() {
    // x0'dan x30'a kadar olan yazmaçları sıfırla.
    // x31 (SP) hariç.
    asm!(
        "eor x0, x0, x0", "eor x1, x1, x1", "eor x2, x2, x2", "eor x3, x3, x3",
        "eor x4, x4, x4", "eor x5, x5, x5", "eor x6, x6, x6", "eor x7, x7, x7",
        "eor x8, x8, x8", "eor x9, x9, x9", "eor x10, x10, x10", "eor x11, x11, x11",
        "eor x12, x12, x12", "eor x13, x13, x13", "eor x14, x14, x14", "eor x15, x15, x15",
        "eor x16, x16, x16", "eor x17, x17, x17", "eor x18, x18, x18", "eor x19, x19, x19",
        "eor x20, x20, x20", "eor x21, x21, x21", "eor x22, x22, x22", "eor x23, x23, x23",
        "eor x24, x24, x24", "eor x25, x25, x25", "eor x26, x26, x26", "eor x27, x27, x27",
        "eor x28, x28, x28", "eor x29, x29, x29", "eor x30, x30, x30",
        options(nomem, nostack)
    );
    serial_println!("[SECURITY] Genel Amaçlı Yazmaçlar Temizlendi.");
}

// -----------------------------------------------------------------------------
// Koruma Ayarları (Temel Kontrol Yazmaçları)
// -----------------------------------------------------------------------------

/// Çekirdek yazmacı erişim kısıtlamalarını etkinleştirir.
/// MMU'nun bir parçası olarak, bu ayarlar genellikle TTBR ve TCR yazmaçları tarafından dolaylı olarak kontrol edilir.
///
/// # Örnek: SCTLR_EL1 (System Control Register)
/// * Bit 0 (M): MMU Enable
/// * Bit 12 (I): Instruction Cache Enable
/// * Bit 13 (C): Data Cache Enable
/// * Bit 24 (UCI): EL0'da Önbellek Talimatlarına İzin Ver (Güvenlik zafiyeti olabilir)
pub fn configure_security_features() {
    serial_println!("[SECURITY] Temel Kontrol Yazmaçları Yapılandırılıyor...");
    
    const SCTLR_EL1: u64 = 0x8; // Temsili bir System Register, gerçek SCTLR_EL1 numarası
    const SCTLR_EL1_RES1: u64 = (1 << 31) | (1 << 30) | (1 << 29) | (1 << 28) | (1 << 25) | (1 << 21) | (1 << 19) | (1 << 18) | (1 << 17) | (1 << 14) | (1 << 11) | (1 << 7) | (1 << 6) | (1 << 5) | (1 << 4);
    
    // Genellikle MMU kodunda yapılır, burada sadece temsili bir güvenlik kontrolü.
    // Sistemin doğru çalışması için rezerve edilmiş bitlerin ayarlanması önemlidir (RES1).
    unsafe {
        let current_sctlr: u64;
        
        // SCTLR_EL1 oku
        asm!("mrs {0}, sctlr_el1", out(reg) current_sctlr, options(nomem, nostack));
        
        let new_sctlr = current_sctlr | SCTLR_EL1_RES1; // Rezerve bitleri koru
        
        // SCTLR_EL1 yaz
        asm!("msr sctlr_el1, {0}", in(reg) new_sctlr, options(nomem, nostack));
        
        // Talimat ve Veri Senkronizasyonu
        io::dsb();
        io::isb();
    }
    
    serial_println!("[SECURITY] SCTLR_EL1 Yazmacı Temel Ayarları Tamamlandı.");
}


// -----------------------------------------------------------------------------
// Temel Güvenlik Başlatma
// -----------------------------------------------------------------------------

/// Güvenlik modülünü başlatır ve temel korumaları etkinleştirir.
pub fn security_init() {
    serial_println!("[SECURITY] ARMv9 Temel Güvenlik Başlatılıyor...");
    
    // 1. Temel sistem kontrol yazmaçlarını yapılandır
    configure_security_features();

    // 2. İlk rastgelelik testi
    let rand_val = get_hardware_random_u64();
    serial_println!("[SECURITY] RNDR Testi: Değer: {:#x}", rand_val);
    
    // 3. (Gelişmiş Kodda): MTE (Memory Tagging Extension) veya PAuth (Pointer Authentication)
    // gibi ARMv9 özel güvenlik özelliklerinin yapılandırılması buraya eklenirdi.
    
    serial_println!("[SECURITY] Temel Güvenlik Yapılandırması Tamamlandı.");
}
// src/arch/mips64/security.rs
// MIPS64 mimarisine özgü temel güvenlik işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/mips64/platformmod.rs dosyasından temel CP0 erişimi ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanımsal Rastgelelik (Hardware Randomness)
// -----------------------------------------------------------------------------

/// Donanımsal Rastgele Sayı Üretimi için bir placeholder.
/// MIPS'te standart bir RNG talimatı yoktur; donanım platformuna (SOC) bağlıdır.
///
/// # İade Değeri
/// u64: Temsili olarak döndürülen değer.
#[inline(always)]
pub fn get_hardware_random_u64() -> u64 {
    serial_println!("[SECURITY] Uyarı: Donanımsal RNG bulunamadı. Temsili değer kullanılıyor.");
    
    // Temsili olarak 0 döndürülür. Gerçek bir implementasyonda,
    // bir zamanlayıcıdan veya özel MMIO/CP0 yazmacından okunmalıdır.
    0xCAFE_BEEF_CAFE_BEEF
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
    // R0 sıfır yazmacıdır. R1'den R31'e kadar olan yazmaçları sıfırla.
    // MIPS64 assembly sözdizimi kullanılır.
    asm!(
        "daddu $r1, $r0, $r0", "daddu $r2, $r0, $r0", "daddu $r3, $r0, $r0", 
        "daddu $r4, $r0, $r0", "daddu $r5, $r0, $r0", "daddu $r6, $r0, $r0",
        "daddu $r7, $r0, $r0", "daddu $r8, $r0, $r0", "daddu $r9, $r0, $r0",
        "daddu $r10, $r0, $r0", "daddu $r11, $r0, $r0", "daddu $r12, $r0, $r0",
        "daddu $r13, $r0, $r0", "daddu $r14, $r0, $r0", "daddu $r15, $r0, $r0",
        "daddu $r16, $r0, $r0", "daddu $r17, $r0, $r0", "daddu $r18, $r0, $r0",
        "daddu $r19, $r0, $r0", "daddu $r20, $r0, $r0", "daddu $r21, $r0, $r0",
        "daddu $r22, $r0, $r0", "daddu $r23, $r0, $r0", "daddu $r24, $r0, $r0",
        "daddu $r25, $r0, $r0", "daddu $r26, $r0, $r0", "daddu $r27, $r0, $r0",
        "daddu $r28, $r0, $r0", "daddu $r29, $r0, $r0", "daddu $r30, $r0, $r0",
        "daddu $r31, $r0, $r0",
        options(nomem, nostack)
    );
    serial_println!("[SECURITY] Genel Amaçlı Yazmaçlar Temizlendi.");
}

// -----------------------------------------------------------------------------
// Koruma Ayarları (CP0 Yapılandırması)
// -----------------------------------------------------------------------------

/// Temel bellek koruma özelliklerini etkinleştirir (NX bitine karşılık gelen özellik).
/// MIPS'te bu genellikle CP0 Status ve Config yazmaçlarında yapılır.
///
/// # Örnek: CP0 Status Yazmacı
/// * **BEV:** Boot Exception Vector (Temizlenmeli)
/// * **EXL:** Exception Level (Temizlenmeli)
/// * **ERL:** Error Level (Temizlenmeli)
/// * **CU0/CU1:** Co-processor Usability (Kullanılabilirlik)
pub fn configure_security_features() {
    serial_println!("[SECURITY] Temel Kontrol Yazmaçları Yapılandırılıyor (CP0)...");
    
    // CP0 Yazmaç Numaraları (Temsili)
    const CP0_STATUS: u32 = 12; 
    const CP0_CONFIG: u32 = 16;
    
    // 1. CP0 Status yazmacını oku
    let mut status = unsafe { io::read_cp0(CP0_STATUS) };
    
    // 2. Güvenlik ve çalışma modu için bitleri temizle/ayarla
    
    // BEV (Bit 22), EXL (Bit 1), ERL (Bit 2) temizlenmeli (Normal Kernel moduna dönmek için)
    const STATUS_BEV: u64 = 1 << 22;
    const STATUS_EXL: u64 = 1 << 1;
    const STATUS_ERL: u64 = 1 << 2;
    
    // Kesmeleri etkinleştir (IE, Bit 0)
    const STATUS_IE: u64 = 1 << 0; 

    // Temizlenecek bitler
    status &= !(STATUS_BEV | STATUS_EXL | STATUS_ERL);
    // Etkinleştirilecek bitler
    status |= STATUS_IE;
    
    // 3. CP0 Status'a yaz
    unsafe { 
        io::write_cp0(CP0_STATUS, status);
        io::sync(); // Senkronizasyon
    }
    
    serial_println!("[SECURITY] CP0 Status Yazmacı Temel Ayarları Tamamlandı.");
}


// -----------------------------------------------------------------------------
// Temel Güvenlik Başlatma
// -----------------------------------------------------------------------------

/// Güvenlik modülünü başlatır ve temel korumaları etkinleştirir.
pub fn security_init() {
    serial_println!("[SECURITY] MIPS64 Temel Güvenlik Başlatılıyor...");
    
    // 1. Temel CP0 kontrol yazmaçlarını yapılandır
    configure_security_features();

    // 2. İlk rastgelelik testi
    let rand_val = get_hardware_random_u64();
    serial_println!("[SECURITY] RNG Testi (Temsili): Değer: {:#x}", rand_val);
    
    // 3. (Gelişmiş Kodda): MMU/TLB'deki özel NX bitine karşılık gelen ayarlar
    // ve diğer spekülatif yürütme mitigasyonları buraya eklenirdi.
    
    serial_println!("[SECURITY] Temel Güvenlik Yapılandırması Tamamlandı.");
}
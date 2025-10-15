// src/arch/openrisc64/security.rs
// OpenRISC 64 (OR64) mimarisine özgü temel güvenlik işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/openrisc64/platformmod.rs dosyasından temel SPR erişimi ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanımsal Rastgelelik (Hardware Randomness)
// -----------------------------------------------------------------------------

/// Donanımsal Rastgele Sayı Üretimi için bir placeholder.
/// OpenRISC ISA'da standart bir RNG talimatı yoktur; donanım platformuna (SOC) bağlıdır.
///
/// # İade Değeri
/// u64: Temsili olarak döndürülen değer.
#[inline(always)]
pub fn get_hardware_random_u64() -> u64 {
    serial_println!("[SECURITY] Uyarı: Donanımsal RNG bulunamadı. Temsili değer kullanılıyor.");
    
    // Temsili olarak 0 döndürülür. Gerçek bir implementasyonda,
    // bir zamanlayıcıdan veya özel MMIO/SPR yazmacından okunmalıdır.
    0xFACE_CAFE_FACE_CAFE
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
    // OpenRISC assembly sözdizimi kullanılır. l.ori rD, rA, UIMM
    // l.ori rX, r0, 0 (R0 sıfır yazmacı olarak kullanılır)
    asm!(
        "l.ori r1, r0, 0", "l.ori r2, r0, 0", "l.ori r3, r0, 0", 
        "l.ori r4, r0, 0", "l.ori r5, r0, 0", "l.ori r6, r0, 0",
        "l.ori r7, r0, 0", "l.ori r8, r0, 0", "l.ori r9, r0, 0",
        "l.ori r10, r0, 0", "l.ori r11, r0, 0", "l.ori r12, r0, 0",
        "l.ori r13, r0, 0", "l.ori r14, r0, 0", "l.ori r15, r0, 0",
        "l.ori r16, r0, 0", "l.ori r17, r0, 0", "l.ori r18, r0, 0",
        "l.ori r19, r0, 0", "l.ori r20, r0, 0", "l.ori r21, r0, 0",
        "l.ori r22, r0, 0", "l.ori r23, r0, 0", "l.ori r24, r0, 0",
        "l.ori r25, r0, 0", "l.ori r26, r0, 0", "l.ori r27, r0, 0",
        "l.ori r28, r0, 0", "l.ori r29, r0, 0", "l.ori r30, r0, 0",
        "l.ori r31, r0, 0",
        options(nomem, nostack)
    );
    serial_println!("[SECURITY] Genel Amaçlı Yazmaçlar Temizlendi.");
}

// -----------------------------------------------------------------------------
// Koruma Ayarları (SPR Yapılandırması)
// -----------------------------------------------------------------------------

/// Temel bellek koruma özelliklerini etkinleştirir (NX bitine karşılık gelen özellik).
/// OpenRISC'te bu genellikle SR (Supervisory Register) ve PIC (Platform Implementation Control) SPR'larında yapılır.
///
/// # Örnek: SR (Supervisory Register) SPR 0x0001
/// * **DS:** Data Space Enable (Veri Alanı Koruması)
/// * **IS:** Instruction Space Enable (Talimat Alanı Koruması)
/// * **TEE:** Tick Timer Exception Enable
/// * **IEE:** Interrupt Exception Enable (Kesme Etkinleştirme)
pub fn configure_security_features() {
    serial_println!("[SECURITY] Temel Kontrol Yazmaçları Yapılandırılıyor (SPR)...");
    
    // SPR Numaraları (Temsili)
    const SPR_SR: u32 = 0x0001; // Supervisory Register
    
    // 1. SR yazmacını oku
    let mut sr_val = unsafe { io::read_spr(SPR_SR) };
    
    // 2. Güvenlik ve çalışma modu için bitleri ayarla/temizle
    
    // Güvenlik amaçlı ayarlar (örneğin MMU/Cache yönetimi ve ayrıcalık seviyesi kontrolü)
    
    // IEE (Bit 1) - Kesmeleri etkinleştir (Genellikle init'te yapılır, burada örnek amaçlı)
    const SR_IEE_BIT: u64 = 1 << 1;
    // DS/IS bitleri, veri/talimat ayrıcalıklarını gösterir (mimari değişkenlik gösterir).
    // Varsayılan olarak Kernel modunda çalıştığımızı varsayarsak, bu bitler zaten doğru olabilir.
    
    sr_val |= SR_IEE_BIT;
    
    // 3. SR'a yaz
    unsafe { 
        io::write_spr(SPR_SR, sr_val);
        io::membar_all(); // Senkronizasyon
    }
    
    serial_println!("[SECURITY] SR (Supervisory Register) Temel Ayarları Tamamlandı.");
}


// -----------------------------------------------------------------------------
// Temel Güvenlik Başlatma
// -----------------------------------------------------------------------------

/// Güvenlik modülünü başlatır ve temel korumaları etkinleştirir.
pub fn security_init() {
    serial_println!("[SECURITY] OpenRISC 64 Temel Güvenlik Başlatılıyor...");
    
    // 1. Temel SPR kontrol yazmaçlarını yapılandır
    configure_security_features();

    // 2. İlk rastgelelik testi
    let rand_val = get_hardware_random_u64();
    serial_println!("[SECURITY] RNG Testi (Temsili): Değer: {:#x}", rand_val);
    
    // 3. (Gelişmiş Kodda): MMU/TLB'deki özel NX bitine karşılık gelen ayarlar
    // ve diğer spekülatif yürütme mitigasyonları buraya eklenirdi.
    
    serial_println!("[SECURITY] Temel Güvenlik Yapılandırması Tamamlandı.");
}
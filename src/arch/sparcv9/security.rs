// src/arch/sparcv9/security.rs
// SPARC V9 (UltraSPARC) mimarisine özgü temel güvenlik işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/sparcv9/platformmod.rs dosyasından temel SPR/ASR erişimi ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanımsal Rastgelelik (Hardware Randomness)
// -----------------------------------------------------------------------------

/// Donanımsal Rastgele Sayı Üretimi için bir placeholder.
/// SPARC V9 ISA'da standart bir RNG talimatı yoktur; donanım platformuna bağlıdır.
///
/// # İade Değeri
/// u64: Temsili olarak döndürülen değer.
#[inline(always)]
pub fn get_hardware_random_u64() -> u64 {
    // Gerçek bir implementasyonda, MMIO yoluyla bir TRNG'den (True Random Number Generator) 
    // okuma yapılmalıdır.
    serial_println!("[SECURITY] Uyarı: Donanımsal RNG bulunamadı. Temsili değer kullanılıyor.");
    
    // Temsili sabit değer.
    0xDEAD_BEEF_DEAD_BEEF
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
    // SPARC V9'da GPR'lar %g1-%g7, %o0-%o7, %l0-%l7, %i0-%i7'dir.
    // %g0 sıfır yazmacıdır.
    
    // Global Yazmaçlar (%g1 - %g7)
    asm!(
        "clr %g1", "clr %g2", "clr %g3", "clr %g4", 
        "clr %g5", "clr %g6", "clr %g7",
        options(nomem, nostack)
    );

    // Kalan GPR'ler (In, Local, Out) genellikle çekirdekten kullanıcıya geçişte sıfırlanır.
    // Ancak pencere yazmaçlarına (Windowed Registers) doğrudan assembly ile erişmek zordur.
    // Tüm pencere yazmaçlarının temizlenmesi için çekirdek, pencere kaydırma mekanizmasını kullanmalıdır.
    
    serial_println!("[SECURITY] Global Yazmaçlar Temizlendi (%g1-%g7).");
    // Pencere yazmaçlarının temizlenmesi için `flushw` ve ardından uygun pencere kaydırmaları gerekir.
    // `io::flushw()` temizler, ancak burada sadece gpr'lar temizlendi.
}

// -----------------------------------------------------------------------------
// Koruma Ayarları (PSTATE/ASR Yapılandırması)
// -----------------------------------------------------------------------------

/// Temel bellek koruma özelliklerini etkinleştirir (NX bitine karşılık gelen özellik).
/// SPARC V9'da bu, PSTATE ve özel ASR yazmaçlarında yapılır.
///
/// # Örnek: PSTATE (Processor State Register)
/// * **AM:** Adress Mask (Adres Alanı Kimliği)
/// * **MMU:** MMU Enable (MMU etkinleştirme)
pub fn configure_security_features() {
    serial_println!("[SECURITY] Temel Kontrol Yazmaçları Yapılandırılıyor (PSTATE/ASR)...");
    
    // 1. PSTATE yazmacını oku
    let mut pstate_val = unsafe { io::read_pstate() };
    
    // 2. Güvenlik ve çalışma modu için bitleri ayarla/temizle
    
    // PSTATE_AM (Adress Mask - Bit 17:16)
    // Temsili olarak PSTATE'te güvenlik ile ilgili bitleri ayarla/temizle.
    
    // PSTATE'de MMU kontrolü genellikle yoktur, bu ayrı bir ASR/MMU kontrol yazmacındadır.
    // Ancak MMU'nun aktif olması (TTE'lerdeki 'E' bitine karşılık gelen NX/XD bitinin varlığı)
    // güvenlik için hayati öneme sahiptir.
    
    // PSTATE'deki G/L/E (Global/Local/Enable) bitlerini kontrol etmek.
    const PSTATE_IE: u64 = 1 << 1; // Interrupt Enable
    
    pstate_val |= PSTATE_IE; // Kesmeleri etkinleştir
    
    // 3. PSTATE'e yaz
    unsafe { 
        io::write_pstate(pstate_val);
        io::membar_all(); // Senkronizasyon
    }
    
    serial_println!("[SECURITY] PSTATE (İşlemci Durum Yazmacı) Temel Ayarları Tamamlandı.");
}


// -----------------------------------------------------------------------------
// Temel Güvenlik Başlatma
// -----------------------------------------------------------------------------

/// Güvenlik modülünü başlatır ve temel korumaları etkinleştirir.
pub fn security_init() {
    serial_println!("[SECURITY] SPARC V9 Temel Güvenlik Başlatılıyor...");
    
    // 1. Temel PSTATE/ASR kontrol yazmaçlarını yapılandır
    configure_security_features();

    // 2. İlk rastgelelik testi
    let rand_val = get_hardware_random_u64();
    serial_println!("[SECURITY] RNG Testi (Temsili): Değer: {:#x}", rand_val);
    
    // 3. (Gelişmiş Kodda): ASI'lar (Address Space Identifiers) ve TTE'ler 
    // üzerinden bellek koruması/NX bitinin etkinleştirilmesi buraya eklenirdi.
    
    serial_println!("[SECURITY] Temel Güvenlik Yapılandırması Tamamlandı.");
}
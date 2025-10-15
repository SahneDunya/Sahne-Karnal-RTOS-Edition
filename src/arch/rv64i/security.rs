// src/arch/rv64i/security.rs
// RISC-V 64 (RV64I) mimarisine özgü temel güvenlik işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/rv64i/platformmod.rs dosyasından temel CSR erişimi ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanımsal Rastgelelik (Hardware Randomness)
// -----------------------------------------------------------------------------

/// Donanımsal Rastgele Sayı Üretimi için bir placeholder.
/// RISC-V ISA'da standart bir RNG talimatı yoktur; donanım platformuna bağlıdır.
///
/// # İade Değeri
/// u64: Temsili olarak döndürülen değer.
#[inline(always)]
pub fn get_hardware_random_u64() -> u64 {
    // Gerçek bir implementasyonda, MMIO yoluyla bir TRNG'den (True Random Number Generator) 
    // okuma yapılmalıdır.
    serial_println!("[SECURITY] Uyarı: Donanımsal RNG bulunamadı. Temsili değer kullanılıyor.");
    
    // Temsili sabit değer.
    0xC0FFEE_C0FFEE_C0FFEE
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
    // x0 sıfır yazmacıdır. x1'den x31'e kadar olan yazmaçları sıfırla.
    // xor talimatı kullanılır: xor xN, xN, xN
    asm!(
        "xor x1, x1, x1", "xor x2, x2, x2", "xor x3, x3, x3", 
        "xor x4, x4, x4", "xor x5, x5, x5", "xor x6, x6, x6",
        "xor x7, x7, x7", "xor x8, x8, x8", "xor x9, x9, x9",
        "xor x10, x10, x10", "xor x11, x11, x11", "xor x12, x12, x12",
        "xor x13, x13, x13", "xor x14, x14, x14", "xor x15, x15, x15",
        "xor x16, x16, x16", "xor x17, x17, x17", "xor x18, x18, x18",
        "xor x19, x19, x19", "xor x20, x20, x20", "xor x21, x21, x21",
        "xor x22, x22, x22", "xor x23, x23, x23", "xor x24, x24, x24",
        "xor x25, x25, x25", "xor x26, x26, x26", "xor x27, x27, x27",
        "xor x28, x28, x28", "xor x29, x29, x29", "xor x30, x30, x30",
        "xor x31, x31, x31",
        options(nomem, nostack)
    );
    serial_println!("[SECURITY] Genel Amaçlı Yazmaçlar Temizlendi.");
}

// -----------------------------------------------------------------------------
// Koruma Ayarları (CSR Yapılandırması)
// -----------------------------------------------------------------------------

/// Temel bellek koruma özelliklerini etkinleştirir (NX bitine karşılık gelen özellik).
/// RISC-V'de bu, genellikle `satp` (Supervisor Address Translation and Protection) 
/// ve `mstatus`/`sstatus` CSR'larında yapılır.
///
/// # Örnek: sstatus (Supervisor Status Register)
/// * **SUM:** Supervisor User Memory Access (Kullanıcı belleğine erişim kontrolü)
/// * **MXR:** Make Executable Readable (Kullanıcı tarafından yürütülebilir sayfaları okunabilir yapar)
pub fn configure_security_features() {
    serial_println!("[SECURITY] Temel Kontrol Yazmaçları Yapılandırılıyor (CSR)...");
    
    // CSR Numaraları (Standart)
    const CSR_SSTATUS: u32 = 0x100; 
    
    // 1. `sstatus` yazmacını oku
    let mut sstatus = unsafe { io::read_csr(CSR_SSTATUS) };
    
    // 2. Güvenlik ve çalışma modu için bitleri ayarla/temizle
    
    // SUM (Bit 18): 0 olması, S-Mode'un U-Mode sayfalarına erişimini engeller (Güvenliği Artırır)
    const SSTATUS_SUM_BIT: u64 = 1 << 18;
    // MXR (Bit 19): 0 olması, Yürütülebilir sayfaların okunmasını engeller (Güvenliği Artırır)
    const SSTATUS_MXR_BIT: u64 = 1 << 19;
    
    // Çekirdek güvenliğini artırmak için SUM ve MXR'ı devre dışı bırak.
    sstatus &= !(SSTATUS_SUM_BIT | SSTATUS_MXR_BIT);
    
    // 3. `sstatus`'a yaz
    unsafe { 
        io::write_csr(CSR_SSTATUS, sstatus);
        io::fence_all(); // Senkronizasyon
    }
    
    serial_println!("[SECURITY] SSTATUS (Supervisor Status) Temel Ayarları Tamamlandı.");
}


// -----------------------------------------------------------------------------
// Temel Güvenlik Başlatma
// -----------------------------------------------------------------------------

/// Güvenlik modülünü başlatır ve temel korumaları etkinleştirir.
pub fn security_init() {
    serial_println!("[SECURITY] RISC-V 64 Temel Güvenlik Başlatılıyor...");
    
    // 1. Temel CSR kontrol yazmaçlarını yapılandır
    configure_security_features();

    // 2. İlk rastgelelik testi
    let rand_val = get_hardware_random_u64();
    serial_println!("[SECURITY] RNG Testi (Temsili): Değer: {:#x}", rand_val);
    
    // 3. (Gelişmiş Kodda): PMP (Physical Memory Protection) ve diğer spekülatif 
    // yürütme mitigasyonları buraya eklenirdi.
    
    serial_println!("[SECURITY] Temel Güvenlik Yapılandırması Tamamlandı.");
}
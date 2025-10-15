// src/arch/powerpc64/security.rs
// PowerPC 64 (PPC64) mimarisine özgü temel güvenlik işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/powerpc64/platformmod.rs dosyasından temel SPR erişimi ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanımsal Rastgelelik (Hardware Randomness)
// -----------------------------------------------------------------------------

/// Donanımsal Rastgele Sayı Üretimi (RNG).
/// PowerPC'de bu genellikle RND talimatı (POWER9+) veya özel MMIO/SPR'lar aracılığıyla sağlanır.
/// Bu kodda, POWER9+ mimarisinde bulunan RND talimatını kullanacağız.
///
/// # İade Değeri
/// u64: Üretilen rastgele sayı.
#[inline(always)]
pub fn get_hardware_random_u64() -> u64 {
    let value: u64;
    
    // RND (Read Random Number) talimatı (POWER9 veya sonrası gerektirir)
    unsafe {
        asm!(
            "rnd {0}",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
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
    // r0'dan r31'e kadar olan yazmaçları sıfırla.
    // r0 (genellikle sıfır/kaynak yazmacı), r1 (SP), r2 (RTOC) hariç tutulabilir.
    // Burada tüm GPR'leri sıfırlayarak maksimum güvenlik sağlıyoruz.
    // rlwinm (Rotate Left Word Immediate then AND with Mask) kullanılır.
    asm!(
        "li r3, 0", "li r4, 0", "li r5, 0", "li r6, 0",
        "li r7, 0", "li r8, 0", "li r9, 0", "li r10, 0",
        "li r11, 0", "li r12, 0", "li r13, 0", "li r14, 0",
        "li r15, 0", "li r16, 0", "li r17, 0", "li r18, 0",
        "li r19, 0", "li r20, 0", "li r21, 0", "li r22, 0",
        "li r23, 0", "li r24, 0", "li r25, 0", "li r26, 0",
        "li r27, 0", "li r28, 0", "li r29, 0", "li r30, 0",
        "li r31, 0",
        // r0, r1, r2'ye dokunmuyoruz. r0 zaten 0'dır, r1 (SP) ve r2 (RTOC) işlevin çalışması için gereklidir.
        // li rX, 0 talimatı rX'i 0'a yükler.
        options(nomem, nostack)
    );
    serial_println!("[SECURITY] Genel Amaçlı Yazmaçlar Temizlendi (r3-r31).");
}

// -----------------------------------------------------------------------------
// Koruma Ayarları (SPR Yapılandırması)
// -----------------------------------------------------------------------------

/// Temel bellek koruma özelliklerini etkinleştirir (NX bitine karşılık gelen özellik).
/// PowerPC'de bu, genellikle MSR (Machine State Register) ve diğer SPR'larda yapılır.
///
/// # Örnek: MSR (Machine State Register)
/// * **PR:** Problem State (Kullanıcı modu)
/// * **DR/IR:** Data/Instruction Relocation (MMU Enable)
/// * **RI:** Recoverable Interrupt (Kurtarılabilir Kesme)
pub fn configure_security_features() {
    serial_println!("[SECURITY] Temel Kontrol Yazmaçları Yapılandırılıyor (MSR)...");
    
    // MSR'ı oku
    let mut msr_val = unsafe { io::read_msr() };
    
    // 2. Güvenlik ve çalışma modu için bitleri ayarla/temizle
    
    // DR (Data Relocation) ve IR (Instruction Relocation) bitleri ayarlanmış olmalıdır
    // ki MMU etkin olsun ve NX/Bellek Koruma işlevleri çalışsın.
    const MSR_DR_BIT: u64 = 1 << 46; // Data Relocation
    const MSR_IR_BIT: u64 = 1 << 47; // Instruction Relocation
    
    // Kernel modunda PR (Problem State) bitini temizle.
    const MSR_PR_BIT: u64 = 1 << 48; // Problem State
    
    // DR ve IR bitlerini etkinleştir (MMU'nun çalışması için gerekli)
    msr_val |= MSR_DR_BIT | MSR_IR_BIT;
    
    // PR bitini temizle (Ayrıcalıklı Kernel Modunda kal)
    msr_val &= !MSR_PR_BIT;
    
    // 3. MSR'a yaz
    unsafe { 
        io::write_msr(msr_val);
        io::membar_all(); // Senkronizasyon
    }
    
    serial_println!("[SECURITY] MSR (Makine Durum Yazmacı) Temel Ayarları Tamamlandı.");
}


// -----------------------------------------------------------------------------
// Temel Güvenlik Başlatma
// -----------------------------------------------------------------------------

/// Güvenlik modülünü başlatır ve temel korumaları etkinleştirir.
pub fn security_init() {
    serial_println!("[SECURITY] PowerPC 64 Temel Güvenlik Başlatılıyor...");
    
    // 1. Temel MSR kontrol yazmaçlarını yapılandır (MMU'yu etkinleştirir/korumayı ayarlar)
    configure_security_features();

    // 2. İlk rastgelelik testi (POWER9+ için)
    let rand_val = get_hardware_random_u64();
    serial_println!("[SECURITY] RNG Testi: Değer: {:#x}", rand_val);
    
    // 3. (Gelişmiş Kodda): Spekülatif yürütme mitigasyonları (örneğin spekülatif dallanma kontrolü)
    // buraya eklenebilir.
    
    serial_println!("[SECURITY] Temel Güvenlik Yapılandırması Tamamlandı.");
}
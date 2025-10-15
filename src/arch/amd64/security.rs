// src/arch/amd64/security.rs
// AMD64 (x86_64) mimarisine özgü temel güvenlik işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/amd64/platformmod.rs dosyasından temel G/Ç (MSR, CR) işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanımsal Rastgelelik (Hardware Randomness)
// -----------------------------------------------------------------------------

/// Donanımsal Rastgele Sayı Üretimi (RDRAND talimatı) ile 64-bit rastgele sayı alır.
///
/// # İade Değeri
/// (u64, bool): Üretilen sayı ve işlemin başarılı olup olmadığını belirten bayrak.
#[inline(always)]
pub fn get_hardware_random_u64() -> (u64, bool) {
    let value: u64;
    let success: u64;
    
    // RDRAND talimatı kullanılır. Başarılıysa carry flag (CF) ayarlanır.
    unsafe {
        asm!(
            "rdrand {0}",
            "setc {1}", // CF set edilmişse '1' döndürür, aksi takdirde '0'
            out(reg) value,
            lateout(reg) success,
            options(nomem, nostack)
        );
    }
    (value, success != 0)
}

// -----------------------------------------------------------------------------
// Yazmaç Temizleme (Register Zeroing)
// -----------------------------------------------------------------------------

/// Genel amaçlı yazmaçları (GPR) bellekten geri dönerken temizler.
///
/// # Güvenlik Notu
/// Spectre v2 mitigasyonu için önemlidir. Çekirdekten kullanıcı alanına
/// geri dönmeden önce hassas verilerin sızmasını önlemeye yardımcı olur.
#[inline(always)]
pub unsafe fn zero_gprs() {
    // Tüm 64-bit genel amaçlı yazmaçları (rsp ve rbp hariç) sıfırla.
    // rsp (stack) ve rbp (frame pointer) mevcut bağlamı tutar, onlara dokunulmaz.
    // Sadece güvenlik amaçlı kullanılmayan yazmaçlar sıfırlanır.
    asm!(
        "xor rax, rax",
        "xor rbx, rbx",
        "xor rcx, rcx",
        "xor rdx, rdx",
        "xor rsi, rsi",
        "xor rdi, rdi",
        "xor r8, r8",
        "xor r9, r9",
        "xor r10, r10",
        "xor r11, r11",
        "xor r12, r12",
        "xor r13, r13",
        "xor r14, r14",
        "xor r15, r15",
        options(nomem, nostack)
    );
    serial_println!("[SECURITY] Genel Amaçlı Yazmaçlar Temizlendi.");
}

// -----------------------------------------------------------------------------
// Koruma Ayarları (NX Bit / XD)
// -----------------------------------------------------------------------------

/// Execute Disable (NX / XD) bitini (No-Execute) etkinleştirir.
/// Bu, veri sayfalarının yürütülmesini engeller, buffer overflow saldırılarını azaltır.
///
/// # MSR: EFER (Extended Feature Enable Register) - 0xC0000080
/// * Bit 11: NXE (No-Execute Enable)
pub fn enable_nx_bit() {
    serial_println!("[SECURITY] Execute Disable (NX) Bitini Etkinleştirme...");

    const EFER_MSR: u32 = 0xC0000080;
    const EFER_NXE_BIT: u64 = 1 << 11; // NXE Biti

    unsafe {
        // 1. EFER MSR'ı oku
        let mut efer = io::rdmsr(EFER_MSR);

        // 2. NXE bitini ayarla (Etkinleştir)
        if (efer & EFER_NXE_BIT) == 0 {
            efer |= EFER_NXE_BIT;

            // 3. EFER MSR'a yaz
            io::wrmsr(EFER_MSR, efer);
            
            serial_println!("[SECURITY] EFER (NXE) başarılıyla ayarlandı.");
        } else {
            serial_println!("[SECURITY] EFER (NXE) zaten etkin.");
        }
    }
}

// -----------------------------------------------------------------------------
// Temel Güvenlik Başlatma
// -----------------------------------------------------------------------------

/// Güvenlik modülünü başlatır ve temel korumaları etkinleştirir.
pub fn security_init() {
    serial_println!("[SECURITY] AMD64 Temel Güvenlik Başlatılıyor...");
    
    // 1. NX/XD (Execute Disable) bitini etkinleştir.
    enable_nx_bit();

    // 2. Diğer güvenlik ayarları (isteğe bağlı ve platforma özgü)
    
    // Örnek: WP (Write Protect) bitini CR0 yazmacında etkinleştirme
    // Bu, çekirdeğin yanlışlıkla kendisine yazmasını engeller.
    const CR0_WP_BIT: u64 = 1 << 16;
    unsafe {
        let mut cr0 = io::read_cr0();
        cr0 |= CR0_WP_BIT;
        io::write_cr0(cr0);
    }
    serial_println!("[SECURITY] CR0 WP (Write Protect) Biti Etkinleştirildi.");
    
    // 3. İlk rastgelelik testi (başarısız olması normaldir, donanım gerektirir)
    let (rand_val, success) = get_hardware_random_u64();
    serial_println!("[SECURITY] RDRAND Testi: Başarılı: {}, Değer: {:#x}", success, rand_val);
    
    serial_println!("[SECURITY] Temel Güvenlik Yapılandırması Tamamlandı.");
}
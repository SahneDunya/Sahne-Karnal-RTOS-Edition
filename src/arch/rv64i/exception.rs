#![allow(non_snake_case)]

use core::arch::asm;
use core::fmt;
use crate::serial_println;

// -----------------------------------------------------------------------------
// HARİCİ MONTAJ DİLİ İŞLEYİCİSİ
// -----------------------------------------------------------------------------

extern "C" {
    /// Tüm istisnalar ve kesmeler için ortak giriş noktası (Montaj kodu).
    /// Bu fonksiyonun adresi, STVEC yazmacına yazılacaktır.
    fn trap_entry();
}

// -----------------------------------------------------------------------------
// 1. İSTİSNA KAYIT YAPILARI
// -----------------------------------------------------------------------------

/// RISC-V 64'te istisna oluştuğunda yığına kaydedilen CPU durumu (Context).
/// Bu yapı, montaj kodunun registerları yığına kaydettiği sıraya UYMALIDIR.
#[repr(C)]
pub struct ExceptionContext {
    // Montaj kodunun kaydettiği tüm genel amaçlı registerlar (x1 - x31)
    // x0 (zero) genellikle kaydedilmez.
    pub gpr: [u64; 31], 
    
    // Kontrol ve Durum Yazmaçları (CSR - Yığına kaydedilmiş kopyalar)
    pub SCAUSE: u64, // İstisnanın nedeni
    pub SEPC: u64, // İstisna Program Sayacı (Exception Program Counter - Dönüş Adresi)
    pub STVAL: u64, // Hatalı Sanal Adres (Bad Virtual Address - Sayfa Hatası vb.)
    pub SSTATUS: u64, // Süpervizör Durum Yazmacı (Kesme durumu vb.)
}

/// İstisna nedenleri (SCAUSE yazmacından alınmıştır).
/// Yüksek bit (63), kesme (1) veya senkron istisna (0) olduğunu belirtir.
#[repr(i64)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ExceptionCause {
    // Senkron İstisnalar (Yüksek Bit 0)
    InstructionPageFault = 12, // Talimat Sayfa Hatası
    LoadPageFault = 13, // Yükleme Sayfa Hatası
    StorePageFault = 15, // Depolama Sayfa Hatası
    EnvironmentCallFromUMode = 8, // U-mode'dan SYSCALL
    EnvironmentCallFromSMode = 9, // S-mode'dan SYSCALL
    InstructionAccessFault = 1, // Talimat Erişim Hatası
    _Unknown(i64),

    // Kesmeler (Yüksek Bit 1)
    SupervisorTimerInterrupt = -1, // S-Mode Zamanlayıcı Kesmesi
    SupervisorSoftwareInterrupt = -2, // S-Mode Yazılım Kesmesi
    SupervisorExternalInterrupt = -3, // S-Mode Harici Kesmesi
}

impl ExceptionCause {
    fn from_scause(scause: u64) -> Self {
        if (scause as i64) < 0 { // Yüksek bit 1 ise (Kesme)
            match scause & 0x7FFFFFFFFFFFFFFF { // Sadece alt 63 bit
                1 => ExceptionCause::SupervisorSoftwareInterrupt,
                5 => ExceptionCause::SupervisorTimerInterrupt,
                9 => ExceptionCause::SupervisorExternalInterrupt,
                _ => ExceptionCause::_Unknown(scause as i64),
            }
        } else { // Senkron İstisna (Yüksek bit 0)
            match scause {
                12 => ExceptionCause::InstructionPageFault,
                13 => ExceptionCause::LoadPageFault,
                15 => ExceptionCause::StorePageFault,
                8 => ExceptionCause::EnvironmentCallFromUMode,
                9 => ExceptionCause::EnvironmentCallFromSMode,
                1 => ExceptionCause::InstructionAccessFault,
                _ => ExceptionCause::_Unknown(scause as i64),
            }
        }
    }
}

// -----------------------------------------------------------------------------
// 2. TEMEL İŞLEYİCİ FONKSİYONLARI (Rust Kodu)
// -----------------------------------------------------------------------------

/// Tüm istisna giriş noktalarından montaj kodu tarafından çağrılan Rust işleyicisi.
#[no_mangle]
pub extern "C" fn generic_trap_handler(context: &mut ExceptionContext) {
    let cause = ExceptionCause::from_scause(context.SCAUSE);
    
    match cause {
        ExceptionCause::SupervisorSoftwareInterrupt | 
        ExceptionCause::SupervisorTimerInterrupt | 
        ExceptionCause::SupervisorExternalInterrupt => 
        {
            // Kesme İşleyicisi
            handle_interrupt(context, cause);
        }
        ExceptionCause::LoadPageFault | ExceptionCause::StorePageFault | ExceptionCause::InstructionPageFault => {
            // Sayfa Hatası İşleyicileri
            serial_println!("\n--- SAYFA HATASI ---");
            serial_println!("Neden: {:?}", cause);
            serial_println!("Hata Adresi (STVAL): {:#x}", context.STVAL);
            serial_println!("SEPC (RIP): {:#x}", context.SEPC);
            
            // Eğer Sayfa Hatası ise, MMU'yu kullanarak çeviri yapmayı deneyin.
            
            panic!("Kritik Sayfa Hatası!");
        }
        ExceptionCause::EnvironmentCallFromUMode | ExceptionCause::EnvironmentCallFromSMode => {
            // Sistem Çağrısı (SYSCALL)
            serial_println!("SYSCALL: Çağrı kodu: {}", context.gpr[10]); // a0 (x10)
            
            // Syscall'dan dönmek için SEPC'yi bir sonraki talimata ilerletmeyi UNUTMAYIN.
            context.SEPC += 4; 
        }
        _ => {
            // Diğer senkron istisnalar
            serial_println!("\n--- KRİTİK İSTİSNA ---");
            serial_println!("SCAUSE: {:#x} ({:?})", context.SCAUSE, cause);
            serial_println!("SEPC: {:#x}", context.SEPC);
            
            panic!("İşlenmemiş İstisna!");
        }
    }
}

/// Donanım ve Yazılım Kesmeleri için özel işleyici.
fn handle_interrupt(_context: &mut ExceptionContext, cause: ExceptionCause) {
    match cause {
        ExceptionCause::SupervisorTimerInterrupt => {
            // CLINT'e zamanlayıcı kesmesini temizle.
            // serial_print!("t");
            // unsafe { arch::rv64i::clint::clear_timer_interrupt(); }
        }
        ExceptionCause::SupervisorExternalInterrupt => {
            // PLIC'ten hangi kesmenin geldiğini oku.
            // serial_print!("e");
            // unsafe { arch::rv64i::plic::handle_external_interrupt(); }
        }
        _ => {
            // Diğerleri...
        }
    }
    // NOT: Kesme dönüşünde SEPC'yi ilerletmeye gerek YOKTUR.
}


// -----------------------------------------------------------------------------
// 3. KESME YÖNETİMİ API'SI
// -----------------------------------------------------------------------------

/// İstisna giriş noktasını ayarlar ve kesmeleri etkinleştirir.
pub fn init_exceptions() {
    unsafe {
        // 1. STVEC yazmacını montaj dilindeki istisna giriş noktasına ayarla.
        // Mod 0 (Doğrudan) - Tüm istisnalar tek bir noktaya sıçrar.
        let entry_addr = trap_entry as u64;
        // STVEC yazmacına yaz
        asm!("csrw stvec, {}", in(reg) entry_addr); 
        
        // 2. SSTATUS yazmacını ayarla (Kesmeleri etkinleştirme).
        // SSTATUS yazmacındaki SIE (Supervisor Interrupt Enable) bitini ayarla.
        let sie_bit = 1 << 1; 
        asm!("csrs sstatus, {}", in(reg) sie_bit); 
        
        // 3. SIE (Supervisor Interrupt Enable) yazmacını ayarla (Hangi kesmelerin S-mode'a gelmesine izin verileceği).
        // SEIE (Harici), STIE (Zamanlayıcı), SSIE (Yazılım)
        let s_interrupts = (1 << 9) | (1 << 5) | (1 << 1); // SEIE | STIE | SSIE
        asm!("csrw sie, {}", in(reg) s_interrupts);
    }
    
    serial_println!("[RV64I] Tuzak (Trap) Yönetimi başlatıldı (S-Mode).");
    serial_println!("[RV64I] Harici, Zamanlayıcı ve Yazılım Kesmeleri etkinleştirildi.");
}
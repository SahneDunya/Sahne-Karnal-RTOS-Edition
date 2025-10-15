// src/arch/openrisc64/time.rs
// OpenRISC 64 (OR64) mimarisine özgü zamanlama (time) işlevleri.

use core::arch::asm;
use crate::serial_println;
// Platforma özel G/Ç fonksiyonları için yer tutucu
use super::platformmod::io; 

/// Timebase Register'dan okunan ham döngü sayısını temsil eder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycles(pub u64);

// Timebase Register (TBR) SPR Numarası
// OpenRISC 1200 spec'e göre Timebase Low/High çifti: 0x011 / 0x012
// 64-bit'te SPR'ler çift olarak okunur, veya doğrudan 64-bit tek bir registerdır.
// OR64'te tek bir 64-bit SPR 0x011 olduğunu varsayıyoruz (TBR).
const SPR_TBR_ID: u32 = 0x011; 

// Global olarak Clock Frekansını (Hz) saklamak için basit bir değişken (Gereklidir)
static mut CLOCK_FREQUENCY_HZ: u64 = 0;

// -----------------------------------------------------------------------------
// Timebase Register (TBR) İşlevleri
// -----------------------------------------------------------------------------

/// Timebase Register (SPR 0x011) yazmacını okur ve ham döngü sayısını döndürür.
///
/// Not: OpenRISC'te SPR okuma talimatı: `l.mfspr rD, rA, spr_id`
/// rA genellikle r0 (sıfır) olmalıdır.
#[inline(always)]
pub fn read_timebase() -> Cycles {
    let high: u32;
    let low: u32;
    
    // OpenRISC 64'te 64-bit okuma varsayımı:
    let cycles: u64;

    unsafe {
        asm!(
            "l.mfspr {0}, r0, {1}", // l.mfspr rD, r0, SPR_TBR_ID
            out(reg) cycles,
            in(reg) SPR_TBR_ID,
            // SPR okuması hafıza erişimi yapmaz
            options(nomem, nostack, preserves_flags)
        );
    }
    
    // Eğer mimari 32-bit'lik iki okuma gerektiriyorsa (l.mfspr rH, r0, TBR_H; l.mfspr rL, r0, TBR_L)
    /*
    unsafe {
        asm!(
            "l.mfspr {0}, r0, {2}", // TBR High (TBR_H = 0x012 varsayalım)
            "l.mfspr {1}, r0, {3}", // TBR Low (TBR_L = 0x011 varsayalım)
            out(reg) high,
            out(reg) low,
            in(reg) 0x012, // Temsili High SPR
            in(reg) 0x011, // Temsili Low SPR
            options(nomem, nostack, preserves_flags)
        );
    }
    let cycles = ((high as u64) << 32) | (low as u64);
    */
    
    Cycles(cycles)
}

/// Çekirdek sayacının frekansını (Hz) döndürür.
pub fn get_frequency() -> u64 {
    unsafe {
        CLOCK_FREQUENCY_HZ
    }
}


// -----------------------------------------------------------------------------
// Temel Zaman İşlevleri
// -----------------------------------------------------------------------------

/// Gecikmeli bir bekleme (busy-wait) döngüsü gerçekleştirir.
pub fn delay_cycles(cycles: u64) {
    if cycles == 0 {
        return;
    }

    let start = read_timebase().0;
    // Taşmayı önlemek için kontrol
    let end = start.checked_add(cycles).unwrap_or(u64::MAX); 

    while read_timebase().0 < end {
        // İşlemciyi uyarmak için `nop` veya `l.nop` kullanılabilir.
        unsafe {
            io::nop(); // Temsili bir NOP
        }
    }
}


/// OpenRISC64 Zamanlama altyapısını başlatır.
///
/// Gerçek bir çekirdekte bu fonksiyon saat frekansını donanımdan okumalıdır.
pub fn initialize_time_system() {
    serial_println!("[TIME] OpenRISC 64 Zamanlama Modülü Başlatılıyor...");
    
    // OpenRISC'te saat frekansı donanım yapılandırmasına bağlıdır. 
    // QEMU veya emülatör ortamları için varsayılan bir değer kullanıyoruz.
    let presumed_freq_hz: u64 = 50_000_000; // Temsili 50 MHz
    
    unsafe {
        CLOCK_FREQUENCY_HZ = presumed_freq_hz;
    }
    
    let current_cycles = read_timebase();
    
    serial_println!("[TIME] Timebase Register Başlangıç Değeri: {:#x}", current_cycles.0);
    serial_println!("[TIME] Varsayılan Frekans (Hz): {}", presumed_freq_hz);
}

// Platforma özel G/Ç fonksiyonları için bir yer tutucu (main.rs veya platformmod.rs'de tamamlanmalıdır)
mod io {
    use core::arch::asm;
    
    #[inline(always)]
    pub unsafe fn nop() {
        // NOP (l.nop) talimatı
        asm!("l.nop", options(nomem, nostack, preserves_flags));
    }
    
    #[inline(always)]
    pub unsafe fn idle() {
        // IDLE (Wait for Interrupt/Event) talimatı (l.wdt gibi)
        asm!("l.nop", options(nomem, nostack, preserves_flags)); // Temsili bir IDLE
    }
}
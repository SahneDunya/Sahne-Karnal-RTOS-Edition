// src/arch/armv9/time.rs
// ARMv9 (aarch64) mimarisine özgü zamanlama (time) işlevleri.

use core::arch::asm;
use crate::serial_println;
// Platforma özel G/Ç fonksiyonları için yer tutucu
use super::platformmod::io; 

/// CNTPCT_EL0'dan okunan ham döngü sayısını temsil eder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycles(pub u64);

// Global olarak frekansı saklamak için basit bir değişken (FreEMM)
static mut COUNTER_FREQUENCY: u64 = 0;

// -----------------------------------------------------------------------------
// System Counter İşlevleri
// -----------------------------------------------------------------------------

/// CNTPCT_EL0 (Physical Counter) yazmacını okur ve ham döngü sayısını döndürür.
#[inline(always)]
pub fn read_cntpct() -> Cycles {
    let cycles: u64;
    
    // MRS Xd, CNTPCT_EL0: CNTPCT_EL0 sistem yazmacının değerini d GPR'a taşır.
    unsafe {
        asm!(
            "mrs {}, cntpct_el0",
            out(reg) cycles,
            // Zamanlama yazmaçları hafıza erişimi yapmaz
            options(nomem, nostack, preserves_flags)
        );
    }
    
    Cycles(cycles)
}

/// CNTFID_EL0 (Counter Frequency) yazmacını okur ve sayacın frekansını (Hz) döndürür.
/// Bu, saniye başına sayım sayısıdır.
#[inline(always)]
pub fn read_frequency() -> u64 {
    let freq: u64;
    
    // MRS Xd, CNTFID_EL0
    unsafe {
        asm!(
            "mrs {}, cntfid_el0",
            out(reg) freq,
            options(nomem, nostack, preserves_flags)
        );
    }
    
    freq
}


/// Sayacın frekansını (Hz) döndürür.
/// initialize_time_system() çağrılana kadar 0 olabilir.
pub fn get_frequency() -> u64 {
    unsafe {
        COUNTER_FREQUENCY
    }
}


// -----------------------------------------------------------------------------
// Temel Zaman İşlevleri
// -----------------------------------------------------------------------------

/// Gecikmeli bir bekleme (busy-wait) döngüsü gerçekleştirir.
///
/// **UYARI:** Bu, çok kaba ve işlemci gücü tüketen bir bekleme yöntemidir. 
/// Sadece bilinen frekanslarda ve kısa gecikmeler için kullanılmalıdır.
pub fn delay_cycles(cycles: u64) {
    if cycles == 0 {
        return;
    }

    let start = read_cntpct().0;
    // Taşmayı önlemek için kontrol
    let end = start.checked_add(cycles).unwrap_or(u64::MAX); 

    while read_cntpct().0 < end {
        // İşlemciyi uyarmak için WFE (Wait For Event) veya NOP kullanılabilir.
        // Basit bir busy-wait için NOP yeterlidir.
        // Gerçek bir bekleme için io::wfi() (Wait For Interrupt) kullanılmalıdır.
        unsafe {
            io::nop(); // Temsili bir NOP
        }
    }
}


/// Sistem Sayacını kullanarak kernel zamanlama altyapısını başlatır.
///
/// Bu fonksiyon, CNTPCT frekansını okur ve global olarak kaydeder.
pub fn initialize_time_system() {
    serial_println!("[TIME] ARMv9 Zamanlama Modülü Başlatılıyor...");
    
    let freq = read_frequency();
    
    unsafe {
        COUNTER_FREQUENCY = freq;
    }
    
    let current_cycles = read_cntpct();
    
    serial_println!("[TIME] CNTPCT Başlangıç Değeri: {:#x}", current_cycles.0);
    serial_println!("[TIME] Sayaç Frekansı (Hz): {}", freq);

    if freq == 0 {
        serial_println!("[TIME] **UYARI:** Sayaç Frekansı okunamadı (0 Hz). Zamanlama hatalı olabilir.");
    }
    
    // Frekans bilindiğinde, saniyeler veya milisaniyeler cinsinden 
    // somut zaman değerleri hesaplanabilir.
}

// Platforma özel G/Ç fonksiyonları için bir yer tutucu (main.rs veya platformmod.rs'de tamamlanmalıdır)
mod io {
    use core::arch::asm;
    
    #[inline(always)]
    pub unsafe fn nop() {
        // NOP (No Operation) talimatı
        asm!("nop", options(nomem, nostack, preserves_flags));
    }
}
// src/arch/rv64i/time.rs
// RISC-V 64 (RV64I) mimarisine özgü zamanlama (time) işlevleri.

use core::arch::asm;
use crate::serial_println;
// Platforma özel G/Ç fonksiyonları için yer tutucu
use super::platformmod::io; 

/// RISC-V `time` CSR'dan okunan ham döngü sayısını temsil eder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycles(pub u64);

// Global olarak Sayaç Frekansını (Hz) saklamak için basit bir değişken
static mut COUNTER_FREQUENCY_HZ: u64 = 0;

// -----------------------------------------------------------------------------
// RISC-V Time Counter İşlevleri
// -----------------------------------------------------------------------------

/// `time` CSR'ını okur ve 64-bit ham döngü sayısını döndürür.
///
/// RV64'te, zaman sayacını okumak için genellikle özel bir talimat olan `rdtime`
/// (veya eşdeğer olan `csrr t0, time`) kullanılır.
#[inline(always)]
pub fn read_time_counter() -> Cycles {
    let cycles: u64;
    
    // RISC-V talimatı: csrr rD, csr (Control and Status Register Read)
    // time CSR'ının numarası 0xC01'dir.
    // Assembly kısa yolu: `rdtime {0}`
    unsafe {
        asm!(
            "rdtime {0}",
            out(reg) cycles,
            // CSR okuması hafıza erişimi yapmaz
            options(nomem, nostack, preserves_flags)
        );
    }
    
    Cycles(cycles)
}

/// Sayaç frekansını (Hz) döndürür.
///
/// Not: RISC-V mimarisi standardı, frekansı bir yazmaçta saklamaz.
/// Bu bilgi platforma özgüdür (örneğin, Machine Mode'daki bir platform yazmacında 
/// veya Memory Mapped I/O bölgesinde). Varsayılan bir değer kullanıyoruz.
pub fn get_frequency() -> u64 {
    unsafe {
        COUNTER_FREQUENCY_HZ
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

    let start = read_time_counter().0;
    // Taşmayı önlemek için kontrol
    let end = start.checked_add(cycles).unwrap_or(u64::MAX); 

    while read_time_counter().0 < end {
        // İşlemciyi uyarmak için `nop` veya `wfi` (Wait For Interrupt) kullanılabilir.
        unsafe {
            io::nop(); // Temsili bir NOP
        }
    }
}


/// RV64 Zamanlama altyapısını başlatır.
///
/// Gerçek bir çekirdekte bu fonksiyon saat frekansını donanımdan okumalıdır.
pub fn initialize_time_system() {
    serial_println!("[TIME] RISC-V 64 Zamanlama Modülü Başlatılıyor...");
    
    // RISC-V'de frekans platforma özgüdür (QEMU/donanım). 
    // Bu frekansın genellikle 10 MHz olduğu varsayılır.
    let presumed_freq_hz: u64 = 10_000_000; // Temsili 10 MHz
    
    unsafe {
        COUNTER_FREQUENCY_HZ = presumed_freq_hz;
    }
    
    let current_cycles = read_time_counter();
    
    serial_println!("[TIME] Time Counter Başlangıç Değeri: {:#x}", current_cycles.0);
    serial_println!("[TIME] Varsayılan Frekans (Hz): {}", presumed_freq_hz);
}

// Platforma özel G/Ç fonksiyonları için bir yer tutucu (main.rs veya platformmod.rs'de tamamlanmalıdır)
mod io {
    use core::arch::asm;
    
    #[inline(always)]
    pub unsafe fn nop() {
        // NOP (No Operation) talimatı
        asm!("nop", options(nomem, nostack, preserves_flags));
    }
    
    #[inline(always)]
    pub unsafe fn wfi() {
        // WFI (Wait For Interrupt) talimatı
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}
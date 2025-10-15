// src/arch/sparcv9/time.rs
// SPARC V9 (UltraSPARC) mimarisine özgü zamanlama (time) işlevleri.

use core::arch::asm;
use crate::serial_println;
// Platforma özel G/Ç fonksiyonları için yer tutucu
use super::platformmod::io; 

/// SPARC V9'un Tick/Clock Register'ından okunan ham döngü sayısını temsil eder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycles(pub u64);

// Global olarak Saat Frekansını (Hz) saklamak için basit bir değişken
static mut CLOCK_FREQUENCY_HZ: u64 = 0;

// SPARC V9'da TICK Register genellikle ASR26 (Ancillary State Register 26) olarak bilinir.
// Yazmaç okuma: `rd %asr26, %rD`
const ASR_TICK_ID: u32 = 26; 

// -----------------------------------------------------------------------------
// SPARC V9 Tick Register İşlevleri
// -----------------------------------------------------------------------------

/// SPARC V9'un TICK Register'ını (ASR26) okur ve 64-bit ham döngü sayısını döndürür.
///
/// Talimat: `rd %asr26, %g_reg` (g-reg yerine rust'ın geçici yazmacı kullanılır)
#[inline(always)]
pub fn read_tick_register() -> Cycles {
    let cycles: u64;
    
    // SPARC V9 talimatı: rd rd, asr (Read from Ancillary State Register)
    // SPARC'ta %g0 - %g7 (r0 - r7), %i0 - %i7 (r24 - r31), vb. kullanılır.
    // Rust'ta `reg` kısıtlayıcısı ile genel amaçlı bir yazmaç seçilir.
    unsafe {
        asm!(
            "rd %asr26, {0}", // rd %asr26, %rD
            out(reg) cycles,
            // ASR okuması hafıza erişimi yapmaz
            options(nomem, nostack, preserves_flags)
        );
    }
    
    Cycles(cycles)
}

/// Saat frekansını (Hz) döndürür.
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

    let start = read_tick_register().0;
    // Taşmayı önlemek için kontrol
    let end = start.checked_add(cycles).unwrap_or(u64::MAX); 

    while read_tick_register().0 < end {
        // SPARC'ta yoğun bekleme için genellikle `nop` kullanılır.
        unsafe {
            io::nop(); // Temsili bir NOP
        }
    }
}


/// SPARC V9 Zamanlama altyapısını başlatır.
///
/// Gerçek bir çekirdekte bu fonksiyon saat frekansını donanımdan okumalıdır.
pub fn initialize_time_system() {
    serial_println!("[TIME] SPARC V9 Zamanlama Modülü Başlatılıyor...");
    
    // SPARC'ta Tick frekansı donanıma bağlıdır. Genellikle işlemci frekansının 
    // bir böleni veya sabit bir değerdir (örneğin 100 MHz).
    // Varsayılan bir değer kullanıyoruz.
    let presumed_freq_hz: u64 = 100_000_000; // Temsili 100 MHz
    
    unsafe {
        CLOCK_FREQUENCY_HZ = presumed_freq_hz;
    }
    
    let current_cycles = read_tick_register();
    
    serial_println!("[TIME] Tick Register Başlangıç Değeri: {:#x}", current_cycles.0);
    serial_println!("[TIME] Varsayılan Frekans (Hz): {}", presumed_freq_hz);

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
    
    #[inline(always)]
    pub unsafe fn idle() {
        // IDLE (Wait for Interrupt/Event) talimatı (SPARC'ta genellikle `sleep` veya `stbar` kullanılır)
        asm!("nop", options(nomem, nostack, preserves_flags)); // Temsili bir IDLE
    }
}
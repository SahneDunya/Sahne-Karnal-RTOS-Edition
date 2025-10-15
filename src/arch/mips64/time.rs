// src/arch/mips64/time.rs
// MIPS 64 (MIPS64) mimarisine özgü zamanlama (time) işlevleri.

use core::arch::asm;
use crate::serial_println;
// Platforma özel G/Ç fonksiyonları için yer tutucu
use super::platformmod::io; 

/// MIPS Count Register'dan okunan ham döngü sayısını temsil eder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycles(pub u64);

// Global olarak CPU frekansını (Hz) saklamak için basit bir değişken (Gereklidir)
static mut CPU_FREQUENCY_HZ: u64 = 0;

// -----------------------------------------------------------------------------
// MIPS Count Register İşlevleri
// -----------------------------------------------------------------------------

/// CP0 (Co-Processor 0) Count Register (Yazmaç 9, Select 0) yazmacını okur
/// ve ham döngü sayısını döndürür.
///
/// Not: MIPS'te 64-bit Count/Compare yazmaçları genellikle 32-bit'lik
/// iki okuma ile birleştirilir. Ancak MIPS64'te doğrudan 64-bit okuma
/// destekleniyorsa (örneğin R4000/MIPS64), bu daha basittir.
#[inline(always)]
pub fn read_count_register() -> Cycles {
    let cycles: u64;
    
    // MIPS talimatı: mfc0 rt, rd, sel (Move From Co-Processor 0)
    // rt: hedef GPR, rd: CP0 yazmacı numarası (9=Count), sel: yazmaç seçimi (0)
    // MIPS64'te 64-bit veri taşımak için varsayılan olarak `mfc0` kullanılır.
    unsafe {
        asm!(
            "mfc0 {0}, $9, 0", // mfc0 rt, Count, 0
            out(reg) cycles,
            // CP0 okuması hafıza erişimi yapmaz
            options(nomem, nostack, preserves_flags)
        );
    }
    
    Cycles(cycles)
}

/// Çekirdek yazmacının frekansını (genellikle CPU frekansının yarısı) döndürür.
/// Gerçek frekans donanımdan öğrenilemediği için sabit bir varsayım kullanılır.
pub fn get_frequency() -> u64 {
    unsafe {
        CPU_FREQUENCY_HZ
    }
}


// -----------------------------------------------------------------------------
// Temel Zaman İşlevleri
// -----------------------------------------------------------------------------

/// Gecikmeli bir bekleme (busy-wait) döngüsü gerçekleştirir.
///
/// **UYARI:** Bu, işlemci gücü tüketen bir bekleme yöntemidir. 
pub fn delay_cycles(cycles: u64) {
    if cycles == 0 {
        return;
    }

    let start = read_count_register().0;
    // Taşmayı önlemek için kontrol
    let end = start.checked_add(cycles).unwrap_or(u64::MAX); 

    while read_count_register().0 < end {
        // İşlemciyi uyarmak için `nop` veya `wait` kullanılabilir.
        unsafe {
            io::nop(); // Temsili bir NOP
        }
    }
}


/// MIPS64 Zamanlama altyapısını başlatır.
///
/// Gerçek bir çekirdekte bu fonksiyon CPU frekansını donanımdan okumalıdır.
pub fn initialize_time_system() {
    serial_println!("[TIME] MIPS 64 Zamanlama Modülü Başlatılıyor...");
    
    // MIPS'te CPU frekansı genellikle sabit bir değere ayarlanmıştır. 
    // QEMU veya emülatör ortamları için varsayılan bir değer kullanıyoruz.
    // Gerçek donanımda, frekans okuma mekanizması çekirdek tarafından uygulanmalıdır
    // (örneğin, PLL/saat yapılandırma yazmaçlarından okuma).
    let presumed_freq_hz: u64 = 100_000_000; // Temsili 100 MHz
    
    // Not: Count register frekansı genellikle CPU frekansının yarısıdır.
    // Ancak basitleştirme için şimdilik CPU frekansı olarak kabul edelim.

    unsafe {
        CPU_FREQUENCY_HZ = presumed_freq_hz;
    }
    
    let current_cycles = read_count_register();
    
    serial_println!("[TIME] Count Register Başlangıç Değeri: {:#x}", current_cycles.0);
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
    pub unsafe fn wait() {
        // WAIT (Wait for Interrupt/Event) talimatı
        asm!("wait", options(nomem, nostack, preserves_flags));
    }
}
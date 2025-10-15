// Bu dosya, `panic!` durumunda çekirdeğin davranışını tanımlar.
// Rust'ın `panic_handler` mekanizması tarafından çağrılır.

use core::arch::asm;
use core::panic::PanicInfo;
use crate::serial_println;
use super::io; // Bariyerler için io modülünü kullanacağız

/// Panik anında işlemcinin durması için kullanılan sonsuz döngü.
///
/// ARMv9'da, sonsuz döngü ve data memory barrier (DMB) kullanımı,
/// bellek operasyonlarının bitmesini garanti eder ve işlemciyi etkin bir şekilde durdurur.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("... İşlemci Durduruluyor (WFI/DMB döngüsü).");
    loop {
        // DMB (Data Memory Barrier): Önceki bellek işlemlerinin tamamlanmasını sağlar.
        io::dmb(); 
        
        // WFI (Wait For Interrupt): Kesme gelene kadar işlemciyi düşük güç modunda bekletir.
        // Kesmeler genellikle devre dışı olduğu için bu sonsuz bir durmadır.
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Çekirdek panik işleyicisi.
///
/// Bu fonksiyon, `panic!` makrosu çağrıldığında tetiklenir.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Seri porta veya ekrana hata mesajını yazdır.
    serial_println!("\n========================================");
    serial_println!("!!! KERNEL PANIC (ARMv9/AARCH64) !!!");
    
    // Panik mesajını yazdır
    if let Some(location) = info.location() {
        serial_println!("Hata Konumu: {}:{}:{}", 
            location.file(), 
            location.line(), 
            location.column()
        );
    } else {
        serial_println!("Hata Konumu: Bilinmiyor");
    }
    
    if let Some(message) = info.message() {
        serial_println!("Hata Mesajı: {}", message);
    } else {
        serial_println!("Hata Mesajı: Yok");
    }

    serial_println!("========================================");

    // 2. Tüm işlemci çekirdeklerini durdur.
    halt_loop();
}
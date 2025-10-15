// Bu dosya, `panic!` durumunda çekirdeğin davranışını tanımlar.
// Rust'ın `panic_handler` mekanizması tarafından çağrılır.

use core::arch::asm;
use core::panic::PanicInfo;
use crate::serial_println;

/// Panik anında işlemcinin durması için kullanılan sonsuz döngü.
///
/// MIPS64'te, kesmeleri devre dışı bırakıp bir 'wait' (veya 'sync' ve 'nop') 
/// döngüsüne girmek, işlemciyi durdurmanın en yaygın yoludur.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("... İşlemci Durduruluyor (WAIT döngüsü).");
    loop {
        // MIPS'te 'wait' talimatı (veya bazen 'pause' veya sadece 'nop')
        // kullanılırken kesmelerin devre dışı olması gerekir.
        unsafe {
            // Tam bir senkronizasyon bariyeri (isteğe bağlı ama iyi pratik)
            asm!("sync", options(nomem, nostack));
            
            // WAIT talimatı (kesme gelene kadar durur)
            // Eğer kesmeler devre dışıysa, bu kalıcı bir durmadır.
            asm!("wait", options(nomem, nostack, preserves_flags));
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
    serial_println!("!!! KERNEL PANIC (MIPS64) !!!");
    
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
    
    // Not: Bu noktada kesmelerin devre dışı olduğundan emin olunmalıdır.
    // Bu genellikle panik öncesi istisna/hata işleyicilerinde yapılır.

    // 2. İşlemciyi durdur.
    halt_loop();
}
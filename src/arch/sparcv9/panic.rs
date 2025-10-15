// Bu dosya, `panic!` durumunda çekirdeğin davranışını tanımlar.
// Rust'ın `panic_handler` mekanizması tarafından çağrılır.

use core::arch::asm;
use core::panic::PanicInfo;
use crate::serial_println;

/// Panik anında işlemcinin durması için kullanılan sonsuz döngü.
///
/// SPARC V9'da, kayıt penceresini temizlemek (`flushw`) ve ardından
/// kesmeler devre dışı bırakılmış bir ortamda 'nop' döngüsüne girmek
/// en güvenli durdurma yöntemidir.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("... İşlemci Durduruluyor (FLUSHW/NOP döngüsü).");
    unsafe {
        // Kayıt pencerelerini ana belleğe veya yığına boşalt.
        // Bu, çekirdek yığınındaki kritik verilerin kaybolmamasını sağlar.
        asm!("flushw", options(nomem, nostack));
    }
    
    loop {
        // En basit ve güvenli bekleme talimatı
        unsafe {
            asm!("nop", options(nomem, nostack, preserves_flags));
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
    serial_println!("!!! KERNEL PANIC (SPARC V9) !!!");
    
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
    
    // 2. İşlemciyi durdur.
    halt_loop();
}
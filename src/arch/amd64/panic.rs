// Bu dosya, `panic!` durumunda çekirdeğin davranışını tanımlar.
// Rust'ın `panic_handler` mekanizması tarafından çağrılır.

use core::arch::asm;
use core::panic::PanicInfo;
use crate::serial_println;

/// Panik anında işlemcinin durması için kullanılan sonsuz döngü ve HLT talimatı.
///
/// Bu fonksiyon, panik durumunda tüm çekirdeklerin sonsuza dek durmasını sağlar.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("... İşlemci Durduruluyor (HLT döngüsü).");
    loop {
        // 'hlt' talimatı, bir kesme gelene kadar işlemciyi duraklatır.
        // Kesmeler genellikle devre dışı bırakıldığı için bu sonsuz bir durmadır.
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
}

/// Çekirdek panik işleyicisi.
///
/// Bu fonksiyon, `panic!` makrosu çağrıldığında tetiklenir.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Seri porta veya ekrana hata mesajını yazdır.
    // Bu, çekirdek hatasının teşhisi için hayati önem taşır.
    serial_println!("\n========================================");
    serial_println!("!!! KERNEL PANIC (x86_64) !!!");
    
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
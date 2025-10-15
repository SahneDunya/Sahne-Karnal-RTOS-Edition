// Bu dosya, `panic!` durumunda çekirdeğin davranışını tanımlar.
// Rust'ın `panic_handler` mekanizması tarafından çağrılır.

use core::arch::asm;
use core::panic::PanicInfo;
use crate::serial_println;

/// Panik anında işlemcinin durması için kullanılan sonsuz döngü.
///
/// OpenRISC'te (OR1K/OR64), işlemciyi durdurmak için genellikle 'l.nop' (No Operation) 
/// talimatı içeren bir sonsuz döngü kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("... İşlemci Durduruluyor (NOP döngüsü).");
    loop {
        // OpenRISC assembly: 'l.nop'
        // Bu, en güvenli duruş yöntemidir, enerji verimliliği donanıma bağlıdır.
        unsafe {
            asm!("l.nop", options(nomem, nostack, preserves_flags));
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
    serial_println!("!!! KERNEL PANIC (OPENRISC64) !!!");
    
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
    // Çok çekirdekli sistemlerde, buraya gelindiğinde diğer çekirdekleri 
    // durdurmak için bir mekanizma (IPI) tetiklenmelidir.
    
    halt_loop();
}
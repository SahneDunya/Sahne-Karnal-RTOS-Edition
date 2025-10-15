// Bu dosya, `panic!` durumunda çekirdeğin davranışını tanımlar.
// Rust'ın `panic_handler` mekanizması tarafından çağrılır.

use core::arch::asm;
use core::panic::PanicInfo;
use crate::serial_println;

/// Panik anında işlemcinin durması için kullanılan sonsuz döngü.
///
/// RISC-V'de, kesmelerin devre dışı bırakıldığı bir ortamda 'wfi' (Wait For Interrupt) 
/// talimatı, işlemciyi düşük güç modunda bekleterek kalıcı bir duruş sağlar.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("... İşlemci Durduruluyor (WFI döngüsü).");
    loop {
        // RISC-V assembly: 'wfi'
        // Bu, kesme gelene kadar işlemciyi duraklatır.
        // Kesmeler devre dışı olduğu için bu sonsuz bir durmadır.
        unsafe {
            asm!("wfi", options(nomem, nostack, preserves_flags));
        }
        // Eğer wfi'dan çıkılırsa (örneğin hatalı konfigürasyon nedeniyle),
        // tekrar wfi'ya girmeden önce bir 'nop' veya 'fence.i' kullanılabilir.
    }
}

/// Çekirdek panik işleyicisi.
///
/// Bu fonksiyon, `panic!` makrosu çağrıldığında tetiklenir.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Seri porta veya ekrana hata mesajını yazdır.
    serial_println!("\n========================================");
    serial_println!("!!! KERNEL PANIC (RISC-V 64) !!!");
    
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
    
    // Not: Bu noktada çekirdek, kesmeleri devre dışı bırakmış olmalıdır.
    // Bu, genellikle panik öncesi istisna/hata işleyicilerinde veya `_start`'ta yapılır.

    // 2. İşlemciyi durdur.
    halt_loop();
}
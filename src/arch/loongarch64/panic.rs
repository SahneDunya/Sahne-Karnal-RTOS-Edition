// Bu dosya, `panic!` durumunda çekirdeğin davranışını tanımlar.
// Rust'ın `panic_handler` mekanizması tarafından çağrılır.

use core::arch::asm;
use core::panic::PanicInfo;
use crate::serial_println;

/// Panik anında işlemcinin durması için kullanılan sonsuz döngü.
///
/// LoongArch'ta (RISC-V/MIPS benzeri), düşük güç moduna geçmek için 
/// genellikle 'idle' veya özel bir sistem çağrısı kullanılır. 
/// Eğer 'idle' mevcut değilse, basit bir 'nop' döngüsü yeterlidir.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("... İşlemci Durduruluyor (NOP/IDLE döngüsü).");
    loop {
        // LoongArch'ta IDLE talimatı (0x018 talimat kodu) veya basit bir NOP döngüsü kullanılır.
        unsafe {
            // Basit bir NOP döngüsü (En güvenli ve evrensel yöntem)
            asm!("nop", options(nomem, nostack, preserves_flags));
            
            // Eğer LoongArch IDLE talimatı mevcutsa ve biliniyorsa bu kullanılabilir:
            // asm!("idle", options(nomem, nostack, preserves_flags));
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
    serial_println!("!!! KERNEL PANIC (LOONGARCH64) !!!");
    
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
    // durdurmak için bir IPI (Inter-Processor Interrupt) göndermek gerekebilir.
    
    halt_loop();
}
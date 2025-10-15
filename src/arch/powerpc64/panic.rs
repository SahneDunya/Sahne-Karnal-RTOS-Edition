// Bu dosya, `panic!` durumunda çekirdeğin davranışını tanımlar.
// Rust'ın `panic_handler` mekanizması tarafından çağrılır.

use core::arch::asm;
use core::panic::PanicInfo;
use crate::serial_println;

/// Panik anında işlemcinin durması için kullanılan sonsuz döngü.
///
/// PowerPC'de, işlemciyi durdurmak için kesmeler devre dışı bırakılır 
/// ve genellikle 'Halt' benzeri bir davranış (e.g., 'wait' talimatı veya 
/// basit bir döngü) kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("... İşlemci Durduruluyor (NOP döngüsü).");
    loop {
        // PowerPC assembly'de doğrudan bir `hlt` yoktur. 
        // Bu yüzden kesmeleri devre dışı bırakmış bir ortamda 
        // güvenli bir bekleme sağlamak için NOP kullanılır.
        unsafe {
            // 'isync' talimatı (Instruction Sync)
            asm!("isync", options(nomem, nostack, preserves_flags));
            
            // Eğer WAIT talimatı ('wait' veya 'slbia' sonrası RFI) güvenle 
            // kullanılabilirse o tercih edilmelidir. Aksi halde NOP yeterlidir.
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
    serial_println!("!!! KERNEL PANIC (POWERPC64) !!!");
    
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
    // Çok çekirdekli sistemlerde (ki PPC64 genellikle öyledir), buraya gelindiğinde 
    // diğer çekirdekleri durdurmak için bir IPI gönderilmesi gerekebilir.
    
    halt_loop();
}
// src/arch/sparcv9/power.rs
// SPARC V9 (UltraSPARC) mimarisine özgü güç yönetimi işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/sparcv9/platformmod.rs dosyasından temel G/Ç işlevlerini içe aktarır.
use super::platformmod::io; 

// Yeniden başlatma ve kapatma için kullanılan MMIO Adresleri/ASI'lar
// NOT: BU ADRESLER KULLANILAN DONANIMA GÖRE DEĞİŞİR (örneğin QEMU/Sun4u veya gerçek Sun)
// Bu değerler sadece temsilidir.
const SYS_CTRL_ADDR: usize = 0x8000_0000; // Temsili Sistem Kontrol Adresi
const SYS_CTRL_ASI: u8 = 0x51;           // Temsili System Control ASI'si
const REBOOT_MAGIC: u64 = 0xFEEDC0DE;     // Temsili Yeniden Başlatma Değeri
const SHUTDOWN_MAGIC: u64 = 0xDEADBEEF;   // Temsili Kapatma Değeri

/// İşlemciyi sonsuz bir bekleme döngüsüne sokar (Genellikle NOP/IDLE döngüsü).
/// Bu, panik anında veya başarısız kapatma/yeniden başlatma sonrası kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("[POWER] İşlemci Durduruldu (IDLE döngüsü).");
    // Kesmeleri devre dışı bırakmış olmalıyız
    loop {
        unsafe {
            io::idle(); // platformmod'dan NOP/IDLE
        }
    }
}

/// Sistemi yeniden başlatmaya çalışır.
///
/// **Yöntem:**
/// 1. MMIO (Memory-Mapped I/O) kullanarak donanım kontrol yazmacına özel bir komut yazmak.
/// 2. Başarısız olursa, sonsuz bekleme döngüsüne girmek.
pub fn system_reboot() -> ! {
    serial_println!("[POWER] Sistemi Yeniden Başlatma Denemesi Başlatılıyor...");
    
    unsafe {
        // 1. Çekirdek kesmelerini devre dışı bırak ve pencereleri temizle
        io::disable_interrupts();
        io::flushw();
        io::membar_all(); // Senkronizasyon
        
        // 2. Yeniden başlatma komutunu özel kontrol yazmacına ASI ile yaz.
        // Genellikle 64-bit yazılır.
        
        // MMIO ile özel ASI kullanarak 64-bit değer yazma:
        let addr = SYS_CTRL_ADDR as *mut u64;
        // Not: write_mmio_64_asi gibi bir fonksiyon platformmod'da olmalıdır.
        // Burada basitçe ptr::write_volatile kullanıyoruz, ancak doğru ASI'yi
        // kullanmak için assembly gerekir. Basitlik için doğrudan ptr::write_volatile kullanıyoruz
        // ve varsayımsal olarak bu adresin doğru davranışı tetiklemesini umuyoruz.
        addr.write_volatile(REBOOT_MAGIC); 

        // Eğer ASI ile yazmak gerekirse: io::write_mmio_64(SYS_CTRL_ADDR, SYS_CTRL_ASI, REBOOT_MAGIC);
        
        io::membar_all(); // Yazma sonrası senkronizasyon
    }

    // 3. Yeniden başlatma başarısız olursa, sonsuza dek dur.
    serial_println!("[POWER] Uyarı: Yeniden Başlatma başarısız oldu, sistem durduruluyor.");
    halt_loop();
}

/// Sistemi tamamen kapatmaya çalışır (Soft-off).
///
/// **Yöntem:**
/// 1. MMIO (Memory-Mapped I/O) kullanarak donanım kontrol yazmacına özel bir komut yazmak.
/// 2. Başarısız olursa, sonsuz bekleme döngüsüne girmek.
pub fn system_shutdown() -> ! {
    serial_println!("[POWER] Sistemi Kapatma Denemesi Başlatılıyor...");

    unsafe {
        // 1. Çekirdek kesmelerini devre dışı bırak ve pencereleri temizle
        io::disable_interrupts();
        io::flushw();
        io::membar_all(); 
        
        // 2. Kapatma komutunu özel kontrol yazmacına ASI ile yaz.
        let addr = SYS_CTRL_ADDR as *mut u64;
        addr.write_volatile(SHUTDOWN_MAGIC);
        
        io::membar_all(); 
    }

    // 3. Kapatma başarısız olursa, sonsuza dek dur.
    serial_println!("[POWER] Uyarı: Kapatma başarısız oldu, sistem durduruluyor.");
    halt_loop();
}

/// İşlemciyi geçici olarak düşük güç moduna alır (Rölantide).
/// Genellikle NOP döngüsü kullanılarak uygulanır.
pub fn system_idle() {
    unsafe {
        // SPARC V9'un düşük güç/bekleme talimatı (l.nop veya platforma özgü bir döngü)
        io::idle(); 
    }
}
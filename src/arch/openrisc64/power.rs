// src/arch/openrisc64/power.rs
// OpenRISC 64 (OR64) mimarisine özgü güç yönetimi işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/openrisc64/platformmod.rs dosyasından temel G/Ç işlevlerini içe aktarır.
use super::platformmod::io; 

// Yeniden başlatma ve kapatma için kullanılan MMIO Adresleri
// NOT: BU ADRESLER KULLANILAN DONANIMA GÖRE DEĞİŞİR (örneğin ORPSoC veya QEMU)
// Bu değerler sadece temsilidir.
const SYS_CTRL_ADDR: usize = 0xF000_0000; // Temsili Sistem Kontrol Adresi
const REBOOT_MAGIC: u64 = 0xDEADBEEF;     // Temsili Yeniden Başlatma Değeri
const SHUTDOWN_MAGIC: u64 = 0xCAFEBABE;   // Temsili Kapatma Değeri

/// İşlemciyi sonsuz bir bekleme döngüsüne sokar (NOP döngüsü).
/// Bu, panik anında veya başarısız kapatma/yeniden başlatma sonrası kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("[POWER] İşlemci Durduruldu (NOP döngüsü).");
    // Kesmeleri devre dışı bırakmış olmalıyız (panic.rs veya platform_init'te)
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
        // 1. Çekirdek kesmelerini devre dışı bırak
        io::disable_interrupts();
        io::membar_all(); // Senkronizasyon
        
        // 2. Yeniden başlatma komutunu özel kontrol yazmacına yaz.
        // OpenRISC 64'te MMIO erişimi genellikle 64-bit'tir.
        let addr = SYS_CTRL_ADDR as *mut u64;
        addr.write_volatile(REBOOT_MAGIC); 
        
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
        // 1. Çekirdek kesmelerini devre dışı bırak
        io::disable_interrupts();
        io::membar_all(); 
        
        // 2. Kapatma komutunu özel kontrol yazmacına yaz.
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
        // OpenRISC'in düşük güç/bekleme talimatı (l.nop)
        io::idle(); 
    }
}
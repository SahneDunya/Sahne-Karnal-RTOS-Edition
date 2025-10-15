// src/arch/powerpc64/power.rs
// PowerPC 64 (PPC64) mimarisine özgü güç yönetimi işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/powerpc64/platformmod.rs dosyasından temel G/Ç işlevlerini içe aktarır.
use super::platformmod::io; 

// Yeniden başlatma ve kapatma için kullanılan MMIO Adresleri veya Komutları
// NOT: BU ADRESLER KULLANILAN DONANIMA GÖRE DEĞİŞİR (örneğin QEMU/pseries, PowerNV)
// Burada, temsili bir MMIO kontrol yazmacı kullanıyoruz.
const POWER_CTRL_ADDR: usize = 0x8000_1000; // Temsili System Control Adresi
const POWER_REBOOT_MAGIC: u64 = 0x55AA55AA;   // Temsili Yeniden Başlatma Değeri
const POWER_SHUTDOWN_MAGIC: u64 = 0xAA55AA55; // Temsili Kapatma Değeri

/// İşlemciyi sonsuz bir bekleme döngüsüne sokar.
/// Bu, panik anında veya başarısız kapatma/yeniden başlatma sonrası kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("[POWER] İşlemci Durduruldu (WAIT döngüsü).");
    // Kesmeleri devre dışı bırakmış olmalıyız (panic.rs veya platform_init'te)
    loop {
        unsafe {
            io::wait(); // platformmod'dan WAIT veya HLT benzeri
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
        // PowerPC 64'te MMIO erişimi genellikle 64-bit'tir.
        let addr = POWER_CTRL_ADDR as *mut u64;
        addr.write_volatile(POWER_REBOOT_MAGIC); 
        
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
        let addr = POWER_CTRL_ADDR as *mut u64;
        addr.write_volatile(POWER_SHUTDOWN_MAGIC);
        
        io::membar_all(); 
    }

    // 3. Kapatma başarısız olursa, sonsuza dek dur.
    serial_println!("[POWER] Uyarı: Kapatma başarısız oldu, sistem durduruluyor.");
    halt_loop();
}

/// İşlemciyi geçici olarak düşük güç moduna alır (Rölantide).
/// Kesme veya olay gelene kadar işlemciyi duraklatır.
pub fn system_idle() {
    unsafe {
        // PowerPC'nin düşük güç bekleme talimatı
        io::wait(); 
    }
}
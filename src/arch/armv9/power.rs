// src/arch/armv9/power.rs
// ARMv9 (aarch64) mimarisine özgü güç yönetimi işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/armv9/platformmod.rs dosyasından temel G/Ç işlevlerini içe aktarır.
use super::platformmod::io; 

// Yeniden başlatma ve kapatma için kullanılan MMIO Adresleri
// NOT: BU ADRESLER KULLANILAN DONANIMA GÖRE DEĞİŞİR (örneğin Raspberry Pi veya QEMU/Virt)
// Bu değerler sadece temsilidir.
const POWER_CTRL_ADDR: usize = 0xFF00_0000;
const POWER_REBOOT_CMD: u64 = 0x52656274; // 'Rebt' temsili komut değeri
const POWER_SHUTDOWN_CMD: u64 = 0x53687464; // 'Shtd' temsili komut değeri

/// İşlemciyi sonsuz bir WFI (Wait For Interrupt) döngüsüne sokar.
/// Bu, panik anında veya başarısız kapatma/yeniden başlatma sonrası kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("[POWER] İşlemci Durduruldu (WFI döngüsü).");
    loop {
        unsafe {
            io::wfi(); // platformmod'dan WFI
        }
    }
}

/// Sistemi yeniden başlatmaya çalışır.
///
/// **Yöntem:**
/// 1. MMIO (Memory-Mapped I/O) kullanarak donanım kontrol yazmacına özel bir komut yazmak.
/// 2. Başarısız olursa, sonsuz WFI döngüsüne girmek.
pub fn system_reboot() -> ! {
    serial_println!("[POWER] Sistemi Yeniden Başlatma Denemesi Başlatılıyor...");
    
    unsafe {
        // 1. Temel bariyerleri çalıştır
        io::dsb(); // Veri senkronizasyonu
        
        // 2. Yeniden başlatma komutunu özel kontrol yazmacına yaz.
        // Genellikle 32-bit veya 64-bit yazılır. Burada 64-bit varsayıyoruz.
        // NOT: Donanıma özgü fonksiyon (write_mmio_64) platformmod'da olmalıdır.
        // (Burada 8-bit olanı kullanmak zorunda kalıyorum, gerçek kodda 64-bit gerekir.)
        
        // Temsili MMIO yazma: (Gerçek kodda 64-bit yazılmalıdır)
        let addr = POWER_CTRL_ADDR as *mut u64;
        addr.write_volatile(POWER_REBOOT_CMD); 
        
        io::dsb(); // Yazma sonrası senkronizasyon
    }

    // 3. Yeniden başlatma başarısız olursa, sonsuza dek dur.
    serial_println!("[POWER] Uyarı: Yeniden Başlatma başarısız oldu, sistem durduruluyor.");
    halt_loop();
}

/// Sistemi tamamen kapatmaya çalışır (Soft-off).
///
/// **Yöntem:**
/// 1. MMIO (Memory-Mapped I/O) kullanarak donanım kontrol yazmacına özel bir komut yazmak.
/// 2. Başarısız olursa, sonsuz WFI döngüsüne girmek.
pub fn system_shutdown() -> ! {
    serial_println!("[POWER] Sistemi Kapatma Denemesi Başlatılıyor...");

    unsafe {
        // 1. Temel bariyerleri çalıştır
        io::dsb(); 
        
        // 2. Kapatma komutunu özel kontrol yazmacına yaz.
        // Temsili MMIO yazma: (Gerçek kodda 64-bit yazılmalıdır)
        let addr = POWER_CTRL_ADDR as *mut u64;
        addr.write_volatile(POWER_SHUTDOWN_CMD);
        
        io::dsb(); 
    }

    // 3. Kapatma başarısız olursa, sonsuza dek dur.
    serial_println!("[POWER] Uyarı: Kapatma başarısız oldu, sistem durduruluyor.");
    halt_loop();
}

/// İşlemciyi geçici olarak düşük güç moduna alır (Kısa döngülerde veya rölantide).
/// Kesme veya olay gelene kadar işlemciyi duraklatır.
pub fn system_idle() {
    unsafe {
        // İşlemciyi bir olay gelene kadar beklet (WFE)
        io::wfe();
        // Alternatif: Kesme gelene kadar beklet (WFI)
        // io::wfi(); 
    }
}
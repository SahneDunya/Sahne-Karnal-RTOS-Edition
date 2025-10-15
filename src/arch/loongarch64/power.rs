// src/arch/loongarch64/power.rs
// LoongArch 64 (LA64) mimarisine özgü güç yönetimi işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/loongarch64/platformmod.rs dosyasından temel G/Ç işlevlerini içe aktarır.
use super::platformmod::io; 

// Yeniden başlatma ve kapatma için kullanılan MMIO Adresleri
// NOT: BU ADRESLER KULLANILAN DONANIMA GÖRE DEĞİŞİR (örneğin QEMU/Virt veya Loongson donanımı)
// Bu değerler sadece temsilidir.
const PMU_CTRL_ADDR: usize = 0x1000_2000; // Temsili PMU/System Control Adresi
const PMU_REBOOT_VAL: u64 = 0xABCD_1234; // Temsili Yeniden Başlatma Değeri
const PMU_SHUTDOWN_VAL: u64 = 0x5678_EF01; // Temsili Kapatma Değeri

/// İşlemciyi sonsuz bir bekleme döngüsüne sokar.
/// Bu, panik anında veya başarısız kapatma/yeniden başlatma sonrası kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("[POWER] İşlemci Durduruldu (IDLE döngüsü).");
    // Kesmeleri devre dışı bırakmış olmalıyız (panic.rs veya platform_init'te)
    loop {
        unsafe {
            io::idle(); // platformmod'dan IDLE veya NOP döngüsü
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
        let addr = PMU_CTRL_ADDR as *mut u64;
        addr.write_volatile(PMU_REBOOT_VAL); 
        
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
        let addr = PMU_CTRL_ADDR as *mut u64;
        addr.write_volatile(PMU_SHUTDOWN_VAL);
        
        io::membar_all(); 
    }

    // 3. Kapatma başarısız olursa, sonsuza dek dur.
    serial_println!("[POWER] Uyarı: Kapatma başarısız oldu, sistem durduruluyor.");
    halt_loop();
}

/// İşlemciyi geçici olarak düşük güç moduna alır (Rölantide).
/// Olay veya kesme gelene kadar işlemciyi duraklatır.
pub fn system_idle() {
    unsafe {
        // LoongArch'un düşük güç bekleme talimatı
        io::idle(); 
    }
}
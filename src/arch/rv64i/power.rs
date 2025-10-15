// src/arch/rv64i/power.rs
// RISC-V 64 (RV64I) mimarisine özgü güç yönetimi işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/rv64i/platformmod.rs dosyasından temel G/Ç işlevlerini içe aktarır.
use super::platformmod::io; 

// SiFive/QEMU platformlarında yaygın olarak kullanılan test/güç kontrol yazmacı adresi.
// Bu adres ve değerler, kullanılan donanıma (örneğin QEMU/virt, SiFive FU740) göre değişebilir.
const SIFIVE_TEST_FAIL: usize = 0x100000;
const SIFIVE_TEST_REBOOT: u32 = 0x3333; // Temsili yeniden başlatma değeri
const SIFIVE_TEST_POWEROFF: u32 = 0x5555; // Temsili kapatma değeri

/// İşlemciyi sonsuz bir WFI (Wait For Interrupt) döngüsüne sokar.
/// Bu, panik anında veya başarısız kapatma/yeniden başlatma sonrası kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("[POWER] İşlemci Durduruldu (WFI döngüsü).");
    // Kesmeleri devre dışı bırakmış olmalıyız
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
    serial_println!("[POWER] Sistemi Yeniden Başlatma Denemesi Başlatılıyor (SiFive Test MMIO)...");
    
    unsafe {
        // 1. Çekirdek kesmelerini devre dışı bırak
        io::disable_interrupts();
        io::fence_all(); // Senkronizasyon
        
        // 2. Yeniden başlatma komutunu özel kontrol yazmacına yaz.
        // SiFive test aygıtı genellikle 32-bit'tir.
        let addr = SIFIVE_TEST_FAIL as *mut u32;
        addr.write_volatile(SIFIVE_TEST_REBOOT); 
        
        io::fence_all(); // Yazma sonrası senkronizasyon
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
    serial_println!("[POWER] Sistemi Kapatma Denemesi Başlatılıyor (SiFive Test MMIO)...");

    unsafe {
        // 1. Çekirdek kesmelerini devre dışı bırak
        io::disable_interrupts();
        io::fence_all(); 
        
        // 2. Kapatma komutunu özel kontrol yazmacına yaz.
        let addr = SIFIVE_TEST_FAIL as *mut u32;
        addr.write_volatile(SIFIVE_TEST_POWEROFF);
        
        io::fence_all(); 
    }

    // 3. Kapatma başarısız olursa, sonsuza dek dur.
    serial_println!("[POWER] Uyarı: Kapatma başarısız oldu, sistem durduruluyor.");
    halt_loop();
}

/// İşlemciyi geçici olarak düşük güç moduna alır (Rölantide).
/// Kesme gelene kadar işlemciyi duraklatır.
pub fn system_idle() {
    unsafe {
        // RISC-V'nin düşük güç bekleme talimatı
        io::wfi(); 
    }
}
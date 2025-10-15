// src/arch/loongarch64/shutdown.rs
// LoongArch 64 (LA64) mimarisine özgü kapatma ve yeniden başlatma işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/loongarch64/platformmod.rs dosyasından temel G/Ç ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// MMIO Kontrol Adresleri (Temsili)
// -----------------------------------------------------------------------------
// Bu adresler, kullanılan donanıma (QEMU veya gerçek SoC) göre değişecektir.
// Temsili bir "System Controller" MMIO adresi ve sihirli değerler kullanalım.
const SYS_CTRL_ADDR: usize = 0x8000_1000; // Temsili System Control Adresi
const REBOOT_MAGIC: u64 = 0x5AA5_5AA5;     // Temsili Yeniden Başlatma Değeri
const POWEROFF_MAGIC: u64 = 0xA55A_A55A;   // Temsili Kapatma Değeri

/// İşlemciyi sonsuz bir bekleme döngüsüne sokar.
/// Başarısız kapatma/yeniden başlatma sonrası kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("[SHUTDOWN] Hata: Kapatma/Yeniden Başlatma başarısız oldu. İşlemci durduruluyor.");
    unsafe {
        io::disable_interrupts();
    }
    loop {
        unsafe {
            // LoongArch'ın bekleme talimatı (genellikle break'e karşılık gelir)
            io::idle(); 
        }
    }
}

// -----------------------------------------------------------------------------
// MMIO Arayüzü
// -----------------------------------------------------------------------------

/// Sistemi MMIO'ya yazarak yeniden başlatmaya çalışır.
fn reboot_via_mmio() -> bool {
    serial_println!("[SHUTDOWN] MMIO ile Yeniden Başlatma Denemesi...");
    
    unsafe {
        let addr = SYS_CTRL_ADDR as *mut u64;
        addr.write_volatile(REBOOT_MAGIC);
        io::membar_all(); // Veri Senkronizasyon Bariyeri
        
        // Başarılı olursa dönülmez
        io::idle(); // Kısa bir bekleme
    }
    false // Başarısız varsayılır
}

/// Sistemi yeniden başlatmaya çalışır.
pub fn system_reboot() -> ! {
    serial_println!("[SHUTDOWN] Sistemi Yeniden Başlatma Başlatılıyor...");
    
    // 1. Kesmeleri devre dışı bırak
    unsafe {
        io::disable_interrupts();
    }
    
    // 2. MMIO ile dene
    reboot_via_mmio(); 
    
    // 3. Tüm yöntemler başarısız olursa
    halt_loop();
}

// -----------------------------------------------------------------------------
// Kapatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi MMIO'ya yazarak kapatmaya çalışır.
fn shutdown_via_mmio() -> bool {
    serial_println!("[SHUTDOWN] MMIO ile Kapatma Denemesi...");
    
    unsafe {
        let addr = SYS_CTRL_ADDR as *mut u64;
        addr.write_volatile(POWEROFF_MAGIC);
        io::membar_all(); // Veri Senkronizasyon Bariyeri

        // Başarılı olursa dönülmez
        io::idle(); // Kısa bir bekleme
    }
    false // Başarısız varsayılır
}

/// Sistemi tamamen kapatmaya çalışır (Soft-off).
pub fn system_shutdown() -> ! {
    serial_println!("[SHUTDOWN] Sistemi Kapatma Başlatılıyor...");
    
    // 1. Kesmeleri devre dışı bırak
    unsafe {
        io::disable_interrupts();
    }

    // 2. MMIO ile kapatmayı dene
    shutdown_via_mmio();
    
    // 3. Fallback: Kapatma başarısız olursa, sonsuza dek dur.
    halt_loop();
}
// src/arch/sparcv9/shutdown.rs
// SPARC V9 (UltraSPARC) mimarisine özgü kapatma ve yeniden başlatma işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/sparcv9/platformmod.rs dosyasından temel G/Ç ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Open Firmware (OF) Sabitleri (Temsili)
// -----------------------------------------------------------------------------
// Open Firmware'a çağrı yapmak için genellikle özel bir Trap (Sistem Çağrısı) kullanılır.
// SPARC'ta bu, genellikle özel bir Traptype ve Register kullanımı gerektirir.

// Temsili Open Firmware Trap Numarası (Gerçek değeri platforma bağlıdır)
const OF_TRAP_NUM: u64 = 0x82; // Genellikle belirli bir trap vektörüne işaret eder.

// Open Firmware'ın standart çağrıları (r2/g2'de tutulur)
const OF_CALL_REBOOT: u64 = 0x5; // Temsili yeniden başlatma çağrısı
const OF_CALL_POWEROFF: u64 = 0x6; // Temsili kapatma çağrısı

// -----------------------------------------------------------------------------
// MMIO Fallback Adresleri (Temsili)
// -----------------------------------------------------------------------------
const SYS_CTRL_ADDR: usize = 0xFF00_1000; // Temsili System Controller Adresi
const REBOOT_MAGIC: u64 = 0x59AA_59AA;     // Temsili Yeniden Başlatma Değeri
const POWEROFF_MAGIC: u64 = 0xAA59_AA59;   // Temsili Kapatma Değeri

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
            // SPARC'ın bekleme talimatı (örn. `unimp` veya `nop` ile loop)
            io::idle(); 
        }
    }
}

// -----------------------------------------------------------------------------
// Open Firmware Arayüzü (Temsili)
// -----------------------------------------------------------------------------

/// Open Firmware hizmet çağrısını (Trap) gerçekleştirir.
/// SPARC'ta bu, genellikle belirli bir yazmaçta (örn. %g1) çağrı tipini 
/// ve özel bir trap'i (`ta` veya `trap`) tetiklemeyi gerektirir.
///
/// # Argümanlar
/// * `func_id`: OF fonksiyonunun kimliği (Örn: OF_CALL_REBOOT)
fn of_call(func_id: u64) {
    // SPARC OF çağrı konvansiyonu basitleştirilmiştir:
    // %g1'de OF fonksiyon kimliği, ardından trap talimatı.
    
    // Not: Gerçek OF çağrısında, çekirdekten OF'ye parametreler (örn. r/g yazmaçları) 
    // aktarılmalı ve Trap Vector'ün doğru ayarlanmış olması gerekir.
    
    unsafe {
        asm!(
            "mov {func_id}, %g1", // %g1'e fonksiyon ID'yi yükle
            "ta {trap_num}",      // Trap on Condition (Trap'i tetikle)
            func_id = in(reg) func_id,
            trap_num = const OF_TRAP_NUM,
            options(nomem, nostack, preserves_flags)
        );
    }
    // Başarılı bir çağrıda buraya asla geri dönülmemelidir.
}

// -----------------------------------------------------------------------------
// Yeniden Başlatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi Open Firmware kullanarak yeniden başlatmaya çalışır.
fn reboot_via_of() -> bool {
    serial_println!("[SHUTDOWN] Open Firmware ile Yeniden Başlatma Denemesi...");
    
    // Open Firmware'a yeniden başlatma çağrısı
    of_call(OF_CALL_REBOOT);
    
    // Başarılı olursa dönülmez
    false 
}

/// Sistemi MMIO'ya yazarak yeniden başlatmaya çalışır (Fallback).
fn reboot_via_mmio() -> bool {
    serial_println!("[SHUTDOWN] MMIO Fallback ile Yeniden Başlatma Denemesi...");
    
    unsafe {
        let addr = SYS_CTRL_ADDR as *mut u64;
        addr.write_volatile(REBOOT_MAGIC);
        io::membar_all(); // Bellek Bariyeri
        
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
    
    // 2. Open Firmware ile dene
    reboot_via_of(); 
    
    // 3. MMIO Fallback ile dene
    reboot_via_mmio();

    // 4. Tüm yöntemler başarısız olursa
    halt_loop();
}

// -----------------------------------------------------------------------------
// Kapatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi Open Firmware kullanarak kapatmaya çalışır.
fn shutdown_via_of() -> bool {
    serial_println!("[SHUTDOWN] Open Firmware ile Kapatma Denemesi...");
    
    // Open Firmware'a kapatma çağrısı
    of_call(OF_CALL_POWEROFF);
    
    // Başarılı olursa dönülmez
    false
}

/// Sistemi MMIO'ya yazarak kapatmaya çalışır (Fallback).
fn shutdown_via_mmio() -> bool {
    serial_println!("[SHUTDOWN] MMIO Fallback ile Kapatma Denemesi...");
    
    unsafe {
        let addr = SYS_CTRL_ADDR as *mut u64;
        addr.write_volatile(POWEROFF_MAGIC);
        io::membar_all(); // Bellek Bariyeri

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

    // 2. Open Firmware ile kapatmayı dene
    shutdown_via_of();
    
    // 3. MMIO Fallback ile dene
    shutdown_via_mmio();
    
    // 4. Fallback: Kapatma başarısız olursa, sonsuza dek dur.
    halt_loop();
}
// src/arch/amd64/shutdown.rs
// AMD64 (x86_64) mimarisine özgü kapatma ve yeniden başlatma işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/amd64/platformmod.rs dosyasından temel G/Ç (I/O) işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// Donanım Adresleri ve Değerler
// -----------------------------------------------------------------------------

// PS/2 Klavye Denetleyicisi Portları (Yeniden Başlatma için en yaygın eski yöntem)
const KBD_CTRL_PORT: u16 = 0x64;
const KBD_CMD_REBOOT: u8 = 0xFE; // Klavye Denetleyicisine Gönderilen Yeniden Başlatma Komutu

// ACPI Power Management Timer Portu (Daha modern sistemler için, genellikle 0xB000'de bulunur)
// Bu sadece temsili bir yöntemdir ve ACPI desteği gerektirir.
const ACPI_PM_CTRL_PORT: u16 = 0xB004; // Temsili bir ACPI PM Portu
const ACPI_POWEROFF_CMD: u8 = 0x20; // Temsili Kapatma Komutu

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
            io::hlt();
        }
    }
}

// -----------------------------------------------------------------------------
// Yeniden Başlatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi, PS/2 Klavye Denetleyicisi kullanarak yeniden başlatmaya çalışır.
/// Bu yöntem çoğu BIOS tabanlı sanal ve fiziksel makinede çalışır.
fn reboot_via_keyboard_controller() -> bool {
    serial_println!("[SHUTDOWN] Klavye Denetleyicisi ile Yeniden Başlatma Denemesi...");
    
    unsafe {
        // Kontrol Portu (0x64) meşgul olana kadar bekle
        for _ in 0..0x10000 {
            if (io::inb(KBD_CTRL_PORT) & 0x02) == 0 {
                break;
            }
        }
        
        // Komutu gönder
        io::outb(KBD_CTRL_PORT, KBD_CMD_REBOOT);
        
        // Komutun başarılı olup olmadığını kontrol edemeyiz, bu yüzden sadece
        // bir süre bekleyip başarısız olduğunu varsayacağız.
        io::pause(500000); // Kısa bir bekleme (temsili)
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
    
    // 2. Klavye denetleyicisi ile dene
    reboot_via_keyboard_controller();
    
    // 3. Triple Fault ile zorla yeniden başlatma (En güvenilir fallback)
    serial_println!("[SHUTDOWN] Triple Fault ile Zorla Yeniden Başlatma Denemesi...");
    unsafe {
        // IDT'yi geçersiz bir adrese ayarla (0)
        let idtr: [u64; 2] = [0, 0];
        asm!("lidt ({0})", in(reg) &idtr as *const _);
        
        // Kesme oluştur (Bu, geçersiz IDT yüzünden Triple Fault'u tetikleyecektir.)
        asm!("int $3"); 
        
        // Normalde buraya asla ulaşılmamalıdır.
    }
    
    // 4. Tüm yöntemler başarısız olursa
    halt_loop();
}

// -----------------------------------------------------------------------------
// Kapatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi, temsili bir ACPI PM kontrol yazmacına yazarak kapatmaya çalışır.
/// Gerçek uygulamada ACPI desteği ve RSPD (Root System Description Pointer) okuması gerekir.
fn shutdown_via_acpi_pm() -> bool {
    serial_println!("[SHUTDOWN] ACPI PM Kontrol Portu ile Kapatma Denemesi...");

    // Not: Bu sadece bir şablondur. ACPI'yi doğru kullanmak çok karmaşıktır.
    // Başarılı olması için donanımın ve ACPI yapılandırmasının desteklemesi gerekir.
    unsafe {
        io::outb(ACPI_PM_CTRL_PORT, ACPI_POWEROFF_CMD);
        io::pause(500000); // Kısa bir bekleme
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

    // 2. ACPI PM ile kapatmayı dene
    shutdown_via_acpi_pm();
    
    // 3. Fallback: Kapatma başarısız olursa, sonsuza dek dur.
    halt_loop();
}
// src/arch/amd64/power.rs
// AMD64 (x86_64) mimarisine özgü güç yönetimi işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/amd64/platformmod.rs dosyasından temel G/Ç işlevlerini içe aktarır.
use super::platformmod::io; 

// Yeniden başlatma ve kapatma için 8042 Klavye Denetleyicisi Portları
const KBD_CMD_PORT: u16 = 0x64; // Klavye Denetleyicisi Komut Portu
const KBD_STATUS_PORT: u16 = 0x64; // Klavye Denetleyicisi Durum Portu
const KBD_DATA_PORT: u16 = 0x60; // Klavye Denetleyicisi Veri Portu

/// 8042 Klavye Denetleyicisinin (PS/2) meşgul olmasını bekler.
/// Okuma/yazma işleminden önce kontrol edilmelidir.
fn kbd_wait() {
    let mut status: u8;
    // Maksimum 100 milisaniye beklemek için 100.000 döngü (temsili)
    for _ in 0..100_000 {
        unsafe {
            // Durum yazmacından oku (Bit 1: Giriş Tamponu Dolu (Meşgul))
            status = io::inb(KBD_STATUS_PORT);
        }
        // Bit 1 (Input Buffer Full - IBF) sıfırsa, kontrolör hazır demektir.
        if (status & 0x02) == 0 {
            return;
        }
    }
    serial_println!("[POWER] Hata: 8042 klavye denetleyicisi zaman aşımına uğradı.");
}

/// Sistemi yeniden başlatmaya çalışır.
///
/// **Yöntemler:**
/// 1. 8042 Klavye Denetleyicisi (En yaygın BIOS tabanlı yöntem).
/// 2. Hata ayıklama/Geliştirme makinesini durdurmak için sonsuz HLT döngüsü (Eğer yeniden başlatma başarısız olursa).
pub fn system_reboot() -> ! {
    serial_println!("[POWER] Sistemi Yeniden Başlatma Denemesi Başlatılıyor (8042 KBD)...");
    
    unsafe {
        // 1. Çekirdek kesmelerini devre dışı bırak
        io::cli();
        
        // 2. 8042 KBD Denetleyicisi aracılığıyla yeniden başlatma komutu gönder
        kbd_wait();
        // Yeniden Başlatma Komutu: Pulse Output bitini ayarlayarak A20'yi sıfırla, ardından CPU'yu sıfırla.
        io::outb(KBD_CMD_PORT, 0xFE); // Yeniden Başlatma Komutu

        // 3. (Gelişmiş Kodda): ACPI veya MSR tabanlı yeniden başlatma denemeleri buraya eklenirdi.
    }

    // 4. Yeniden başlatma başarısız olursa, sonsuza dek dur.
    serial_println!("[POWER] Uyarı: Yeniden Başlatma başarısız oldu, sistem durduruluyor.");
    halt_loop();
}

/// Sistemi tamamen kapatmaya çalışır (Halt).
///
/// **Yöntemler:**
/// 1. Sonsuz HLT döngüsü (En temel durdurma).
/// 2. (Gelişmiş Kodda): ACPI veya güç portu tabanlı kapatma denemeleri buraya eklenirdi.
pub fn system_shutdown() -> ! {
    serial_println!("[POWER] Sistemi Kapatma Denemesi Başlatılıyor (HLT)...");

    unsafe {
        // 1. Çekirdek kesmelerini devre dışı bırak
        io::cli();
    }

    // 2. Sistem G/Ç ayarlarını veya ACPI durumunu değiştirmeden
    // en güvenli ve basit durdurma mekanizması:
    halt_loop();
}

/// İşlemciyi sonsuz bir HLT (Halt) döngüsüne sokar.
/// Bu, panik anında veya başarısız kapatma/yeniden başlatma sonrası kullanılır.
#[inline(always)]
fn halt_loop() -> ! {
    serial_println!("[POWER] İşlemci Durduruldu.");
    loop {
        unsafe {
            io::hlt();
        }
    }
}
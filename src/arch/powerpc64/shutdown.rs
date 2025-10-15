// src/arch/powerpc64/shutdown.rs
// PowerPC 64 (PPC64) mimarisine özgü kapatma ve yeniden başlatma işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/powerpc64/platformmod.rs dosyasından temel G/Ç ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// RTAS (Run-Time Abstraction Services) Sabitleri (Temsili)
// -----------------------------------------------------------------------------
// RTAS'ı kullanmak için Firmware'dan fonksiyon çağrı adreslerini (token) okumamız gerekir.
// Burada, çağrı mekanizmasını göstermek için temsili token'lar kullanacağız.

const RTAS_TOKEN_REBOOT: u32 = 0xDEB007; // Temsili Yeniden Başlatma RTAS Token'ı
const RTAS_TOKEN_SHUTDOWN: u32 = 0xDED0FF; // Temsili Kapatma RTAS Token'ı
const RTAS_ADDR: u64 = 0x80000000;      // Temsili RTAS Giriş Noktası Adresi
const RTAS_SUCCESS: i32 = 0;

// -----------------------------------------------------------------------------
// MMIO Fallback Adresleri (Temsili)
// -----------------------------------------------------------------------------
const SYS_CTRL_ADDR: usize = 0xF000_1000; // Temsili System Controller Adresi
const REBOOT_MAGIC: u64 = 0x5EEB007;     // Temsili Yeniden Başlatma Değeri
const POWEROFF_MAGIC: u64 = 0xDEEADFFF;   // Temsili Kapatma Değeri

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
            // PowerPC'nin bekleme talimatı (wait instruction)
            io::wait(); 
        }
    }
}

// -----------------------------------------------------------------------------
// RTAS Arayüzü (Temsili)
// -----------------------------------------------------------------------------

/// RTAS hizmet çağrısını (sc) gerçekleştirir.
/// RTAS çağrıları, r3-r10 yazmaçlarını kullanarak yapılır.
///
/// # Argümanlar
/// * `token`: RTAS fonksiyonunun kimliği (Token)
/// * `nargs`: Argüman sayısı (token hariç)
/// * `nret`: Dönüş değeri sayısı
/// * `...`: Argümanlar ve dönüş değerleri
///
/// # İade Değeri
/// i32: RTAS çağrısının dönüş kodu (r3'te döner)
#[allow(unused_assignments)]
fn rtas_call(token: u32, nargs: u32, nret: u32, arg1: u64) -> i32 {
    let mut ret_code: u64 = 0;
    
    // PowerPC RTAS çağrı konvansiyonu:
    // r3: token, r4: nargs, r5: nret, r6: arg1, r7: ret_code_ptr (r3'e döner)
    // RTAS_ADDR global bir adres olmalıdır.
    
    // Not: Gerçek RTAS'ta, argümanlar için özel bir alan hazırlanmalı ve
    // r6'da bu alana bir işaretçi verilmelidir. Burada basitleştirilmiştir.
    
    unsafe {
        asm!(
            "li r12, 0", // r12 = 0 (r12 RTAS/Hypervisor çağrısı için kullanılır)
            "sc",        // System Call (RTAS'ı çağırır)
            in("r3") token,
            in("r4") nargs,
            in("r5") nret,
            in("r6") arg1,
            // r3 RTAS'tan dönüş değeri (ret_code) olarak kullanılır.
            out("r3") ret_code, 
            options(nostack, preserves_flags)
        );
    }
    
    ret_code as i32
}

// -----------------------------------------------------------------------------
// Yeniden Başlatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi RTAS kullanarak yeniden başlatmaya çalışır.
fn reboot_via_rtas() -> bool {
    serial_println!("[SHUTDOWN] RTAS ile Yeniden Başlatma Denemesi...");
    
    // RTAS token'ı, 0 argüman, 1 dönüş değeri (dönüş kodu)
    let result = rtas_call(RTAS_TOKEN_REBOOT, 0, 1, 0);
    
    if result == RTAS_SUCCESS {
        // Başarılı olursa buraya asla dönmemelidir
        true 
    } else {
        serial_println!("[SHUTDOWN] RTAS Yeniden Başlatma Hata Kodu: {}", result);
        false
    }
}

/// Sistemi MMIO'ya yazarak yeniden başlatmaya çalışır (Fallback).
fn reboot_via_mmio() -> bool {
    serial_println!("[SHUTDOWN] MMIO Fallback ile Yeniden Başlatma Denemesi...");
    
    unsafe {
        let addr = SYS_CTRL_ADDR as *mut u64;
        addr.write_volatile(REBOOT_MAGIC);
        io::membar_all(); // Data Synchronization Barrier (sync)
        
        // Başarılı olursa dönülmez
        io::wait(); // Kısa bir bekleme
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
    
    // 2. RTAS ile dene
    reboot_via_rtas(); 
    
    // 3. MMIO Fallback ile dene
    reboot_via_mmio();

    // 4. Tüm yöntemler başarısız olursa
    halt_loop();
}

// -----------------------------------------------------------------------------
// Kapatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi RTAS kullanarak kapatmaya çalışır.
fn shutdown_via_rtas() -> bool {
    serial_println!("[SHUTDOWN] RTAS ile Kapatma Denemesi...");
    
    let result = rtas_call(RTAS_TOKEN_SHUTDOWN, 0, 1, 0);
    
    if result == RTAS_SUCCESS {
        // Başarılı olursa buraya asla dönmemelidir
        true 
    } else {
        serial_println!("[SHUTDOWN] RTAS Kapatma Hata Kodu: {}", result);
        false
    }
}

/// Sistemi MMIO'ya yazarak kapatmaya çalışır (Fallback).
fn shutdown_via_mmio() -> bool {
    serial_println!("[SHUTDOWN] MMIO Fallback ile Kapatma Denemesi...");
    
    unsafe {
        let addr = SYS_CTRL_ADDR as *mut u64;
        addr.write_volatile(POWEROFF_MAGIC);
        io::membar_all(); // Data Synchronization Barrier

        // Başarılı olursa dönülmez
        io::wait(); // Kısa bir bekleme
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

    // 2. RTAS ile kapatmayı dene
    shutdown_via_rtas();
    
    // 3. MMIO Fallback ile dene
    shutdown_via_mmio();
    
    // 4. Fallback: Kapatma başarısız olursa, sonsuza dek dur.
    halt_loop();
}
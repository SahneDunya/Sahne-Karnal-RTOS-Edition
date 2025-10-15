// src/arch/armv9/shutdown.rs
// ARMv9 (aarch64) mimarisine özgü kapatma ve yeniden başlatma işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/armv9/platformmod.rs dosyasından temel G/Ç ve bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// PSCI (Power State Coordination Interface) Sabitleri
// -----------------------------------------------------------------------------
// PSCI fonksiyonlarını SMC (Secure Monitor Call) veya HVC (Hypervisor Call) ile çağırırız.
// Genellikle Kernel (EL1) SMC kullanır.

// SMC çağrı numaraları (Hizmet Kimlikleri)
const PSCI_FN_SYSTEM_OFF: u64 = 0x84000008; // Kapatma (Shutdown)
const PSCI_FN_SYSTEM_RESET: u64 = 0x84000009; // Yeniden Başlatma (Reboot)
const PSCI_SUCCESS: i32 = 0;

// -----------------------------------------------------------------------------
// MMIO Fallback Adresleri (Örn: QEMU 'virt' platformu için)
// -----------------------------------------------------------------------------
// Gerçek donanımda bu, SoC'ye özel olurdu. QEMU 'virt' için genellikle bir 
// "test" aygıtı veya bir seri port üzerinden kapatma/yeniden başlatma denenir.
const QEMU_TEST_ADDR: usize = 0x100000;
const QEMU_REBOOT_MAGIC: u32 = 0x5555;
const QEMU_POWEROFF_MAGIC: u32 = 0x7777; // Temsili kapatma değeri

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
            // Wait For Interrupt (WFI) ile düşük güçte beklet
            io::wfi(); 
        }
    }
}

// -----------------------------------------------------------------------------
// PSCI Arayüzü
// -----------------------------------------------------------------------------

/// PSCI hizmet çağrısını (SMC) gerçekleştirir.
/// PSCI hizmetleri genellikle 64-bit çağrılar kullanır.
///
/// # Argümanlar
/// * `function_id`: PSCI fonksiyonunun kimliği (Örn: PSCI_FN_SYSTEM_RESET)
/// * `arg0` - `arg3`: Fonksiyona iletilen argümanlar (çoğu zaman 0)
///
/// # İade Değeri
/// i32: PSCI çağrısının dönüş kodu.
fn psci_call(function_id: u64, arg0: u64, arg1: u64, arg2: u64) -> i32 {
    let ret: u64;
    
    // x0'da fonksiyon ID'si ve argümanlar x1-x3'te olmalıdır.
    unsafe {
        asm!(
            "smc #0", // Secure Monitor Call
            in("x0") function_id,
            in("x1") arg0,
            in("x2") arg1,
            in("x3") arg2,
            out("x0") ret,
            options(nomem, nostack, preserves_flags)
        );
    }
    ret as i32
}

// -----------------------------------------------------------------------------
// Yeniden Başlatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi PSCI kullanarak yeniden başlatmaya çalışır.
fn reboot_via_psci() -> bool {
    serial_println!("[SHUTDOWN] PSCI ile Yeniden Başlatma Denemesi...");
    
    // PSCI_FN_SYSTEM_RESET çağrısı
    let result = psci_call(PSCI_FN_SYSTEM_RESET, 0, 0, 0);
    
    if result == PSCI_SUCCESS {
        // Başarılı olursa buraya asla dönmemelidir
        true 
    } else {
        serial_println!("[SHUTDOWN] PSCI Yeniden Başlatma Hata Kodu: {}", result);
        false
    }
}

/// Sistemi MMIO'ya yazarak yeniden başlatmaya çalışır (Fallback).
fn reboot_via_mmio() -> bool {
    serial_println!("[SHUTDOWN] MMIO Fallback ile Yeniden Başlatma Denemesi...");
    
    unsafe {
        let addr = QEMU_TEST_ADDR as *mut u32;
        addr.write_volatile(QEMU_REBOOT_MAGIC);
        io::dsb(); // Data Synchronization Barrier
        
        // Başarılı olursa dönülmez
        io::wfi(); // Kısa bir bekleme
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
    
    // 2. PSCI ile dene
    reboot_via_psci(); 
    
    // 3. MMIO Fallback ile dene
    reboot_via_mmio();

    // 4. Tüm yöntemler başarısız olursa
    halt_loop();
}

// -----------------------------------------------------------------------------
// Kapatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi PSCI kullanarak kapatmaya çalışır.
fn shutdown_via_psci() -> bool {
    serial_println!("[SHUTDOWN] PSCI ile Kapatma Denemesi...");
    
    // PSCI_FN_SYSTEM_OFF çağrısı
    let result = psci_call(PSCI_FN_SYSTEM_OFF, 0, 0, 0);
    
    if result == PSCI_SUCCESS {
        // Başarılı olursa buraya asla dönmemelidir
        true 
    } else {
        serial_println!("[SHUTDOWN] PSCI Kapatma Hata Kodu: {}", result);
        false
    }
}

/// Sistemi MMIO'ya yazarak kapatmaya çalışır (Fallback).
fn shutdown_via_mmio() -> bool {
    serial_println!("[SHUTDOWN] MMIO Fallback ile Kapatma Denemesi...");
    
    unsafe {
        let addr = QEMU_TEST_ADDR as *mut u32;
        addr.write_volatile(QEMU_POWEROFF_MAGIC);
        io::dsb(); // Data Synchronization Barrier

        // Başarılı olursa dönülmez
        io::wfi(); // Kısa bir bekleme
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

    // 2. PSCI ile kapatmayı dene
    shutdown_via_psci();
    
    // 3. MMIO Fallback ile dene
    shutdown_via_mmio();
    
    // 4. Fallback: Kapatma başarısız olursa, sonsuza dek dur.
    halt_loop();
}
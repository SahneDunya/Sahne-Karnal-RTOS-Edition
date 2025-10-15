// src/arch/rv64i/shutdown.rs
// RISC-V 64 (RV64I) mimarisine özgü kapatma ve yeniden başlatma işlevleri.

use core::arch::asm;
use crate::serial_println;
// arch/rv64i/platformmod.rs dosyasından temel bariyer işlevlerini içe aktarır.
use super::platformmod::io; 

// -----------------------------------------------------------------------------
// SBI (Supervisor Binary Interface) Sabitleri
// -----------------------------------------------------------------------------
// SBI, a7'de EID (Extension ID) ve a6'da FID (Function ID) kullanır.

// EID (Extension IDs)
const EID_SRST: u64 = 0x53525354; // System Reset Extension (SRST)

// FID (Function IDs) - SRST Extension
const SRST_FUNCTION_SYSTEM_RESET: u64 = 0x0;

// Reset Türleri (Reset Type) - a0
const SRST_TYPE_SHUTDOWN: u64 = 0x0;
const SRST_TYPE_COLD_REBOOT: u64 = 0x1;
const SRST_TYPE_WARM_REBOOT: u64 = 0x2;

// Reset Nedenleri (Reset Reason) - a1
const SRST_REASON_NONE: u64 = 0x0;
const SRST_REASON_SYSTEM_FAILURE: u64 = 0x1;

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
// SBI Arayüzü
// -----------------------------------------------------------------------------

/// SBI hizmet çağrısını (ecall) gerçekleştirir.
///
/// # Argümanlar
/// * `eid`: SBI Extension ID (a7)
/// * `fid`: SBI Function ID (a6)
/// * `arg0` - `arg5`: Fonksiyona iletilen argümanlar (a0-a5)
fn sbi_call(eid: u64, fid: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) {
    // SBI çağrısının geri dönüş değeri (a0 ve a1) normalde alınır,
    // ancak sistem reset/kapatma çağrılarında geri dönüş beklenmez.
    unsafe {
        asm!(
            "ecall", // Environment Call (M-Mode'a geçer)
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            in("a4") arg4,
            in("a5") arg5,
            options(nomem, nostack, preserves_flags)
        );
    }
    // Başarılı bir çağrıda buraya asla geri dönülmemelidir.
}

// -----------------------------------------------------------------------------
// Yeniden Başlatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi SBI kullanarak yeniden başlatmaya çalışır.
/// SBI SRST (System Reset) uzantısını kullanır.
pub fn system_reboot() -> ! {
    serial_println!("[SHUTDOWN] Sistemi Yeniden Başlatma Başlatılıyor (SBI COLD)...");
    
    // 1. Kesmeleri devre dışı bırak
    unsafe {
        io::disable_interrupts();
    }
    
    // 2. SBI SRST (System Reset) çağrısı
    // a0: Reset Type (COLD_REBOOT), a1: Reset Reason (NONE)
    sbi_call(
        EID_SRST, 
        SRST_FUNCTION_SYSTEM_RESET, 
        SRST_TYPE_COLD_REBOOT, 
        SRST_REASON_NONE, 
        0, 0, 0, 0
    ); 
    
    // 3. SBI başarısız olursa
    halt_loop();
}

// -----------------------------------------------------------------------------
// Kapatma İşlevleri
// -----------------------------------------------------------------------------

/// Sistemi tamamen kapatmaya çalışır (Soft-off) via SBI.
pub fn system_shutdown() -> ! {
    serial_println!("[SHUTDOWN] Sistemi Kapatma Başlatılıyor (SBI SHUTDOWN)...");
    
    // 1. Kesmeleri devre dışı bırak
    unsafe {
        io::disable_interrupts();
    }

    // 2. SBI SRST (System Reset) çağrısı
    // a0: Reset Type (SHUTDOWN), a1: Reset Reason (NONE)
    sbi_call(
        EID_SRST, 
        SRST_FUNCTION_SYSTEM_RESET, 
        SRST_TYPE_SHUTDOWN, 
        SRST_REASON_NONE, 
        0, 0, 0, 0
    );
    
    // 3. Fallback: Kapatma başarısız olursa, sonsuza dek dur.
    halt_loop();
}
use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;

// -----------------------------------------------------------------------------
// 1. MEMORY-MAPPED I/O (MMIO) İŞLEMLERİ
// -----------------------------------------------------------------------------

// OpenRISC'te donanım erişimi tamamen MMIO yoluyla yapılır.

/// Belirtilen bellek adresinden (MMIO) bir u8 okur.
///
/// # Güvenlik Notu
/// Doğrudan donanım yazmaçlarına erişir ve yetkilendirme gerektirir.
#[inline(always)]
pub unsafe fn mmio_read_u8(addr: usize) -> u8 {
    read_volatile(addr as *const u8)
}

/// Belirtilen bellek adresine (MMIO) bir u8 yazar.
#[inline(always)]
pub unsafe fn mmio_write_u8(addr: usize, value: u8) {
    write_volatile(addr as *mut u8, value)
}

/// Belirtilen bellek adresinden (MMIO) bir u32 okur.
#[inline(always)]
pub unsafe fn mmio_read_u32(addr: usize) -> u32 {
    read_volatile(addr as *const u32)
}

/// Belirtilen bellek adresine (MMIO) bir u32 yazar.
#[inline(always)]
pub unsafe fn mmio_write_u32(addr: usize, value: u32) {
    write_volatile(addr as *mut u32, value)
}

/// Belirtilen bellek adresinden (MMIO) bir u64 okur.
#[inline(always)]
pub unsafe fn mmio_read_u64(addr: usize) -> u64 {
    read_volatile(addr as *const u64)
}

/// Belirtilen bellek adresine (MMIO) bir u64 yazar.
#[inline(always)]
pub unsafe fn mmio_write_u64(addr: usize, value: u64) {
    write_volatile(addr as *mut u64, value)
}

// -----------------------------------------------------------------------------
// 2. BELLEK SENKRONİZASYON BARİYERİ
// -----------------------------------------------------------------------------

// OpenRISC'te bellek erişimlerinin doğru sırasını sağlamak için 'l.msync' talimatı kullanılır.

/// Bellek Senkronizasyon Bariyeri (MSYNC - Memory Synchronization).
/// Bu talimattan önce başlayan tüm bellek erişimlerinin (özellikle MMIO için kritik), 
/// bu talimattan sonra başlayan herhangi bir erişimden önce tamamlanmasını sağlar.
#[inline(always)]
pub fn msync() {
    unsafe {
        // OpenRISC assembly: 'l.msync'
        asm!("l.msync", options(nomem, nostack)); 
    }
}

// -----------------------------------------------------------------------------
// 3. YARDIMCI FONKSİYONLAR
// -----------------------------------------------------------------------------

/// KISA I/O GECİKMESİ
/// OpenRISC'te MMIO gecikmesi için genellikle bir bellek senkronizasyon bariyeri kullanılır.
#[inline(always)]
pub fn io_wait() {
    msync();
}
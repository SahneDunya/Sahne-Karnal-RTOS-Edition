use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;

// -----------------------------------------------------------------------------
// 1. MEMORY-MAPPED I/O (MMIO) İŞLEMLERİ
// -----------------------------------------------------------------------------

// MIPS'te donanım erişimi tamamen MMIO yoluyla yapılır.
// Geleneksel olarak KSEG1 bölgesi önbelleksiz olarak MMIO için kullanılır.

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
// 2. BELLEK VE TALİMAT BARİYERİ (SENKRONİZASYON)
// -----------------------------------------------------------------------------

// MIPS'te bellek erişimlerinin doğru sırasını sağlamak için 'sync' talimatı kullanılır.
// Bu, hem veri (store/load) hem de talimat (instruction) akışını etkileyebilir.

/// Bellek Senkronizasyon Bariyeri (SYNC).
/// Bu talimattan önce başlayan tüm bellek erişimlerinin tamamlanmasını sağlar 
/// ve genellikle MMIO yazma işlemlerinden sonra kullanılır.
#[inline(always)]
pub fn sync() {
    unsafe {
        // MIPS assembly: 'sync'
        // '0' seçeneği genellikle tam sistem bariyeri anlamına gelir.
        asm!("sync 0", options(nomem, nostack)); 
    }
}

// -----------------------------------------------------------------------------
// 3. YARDIMCI FONKSİYONLAR
// -----------------------------------------------------------------------------

/// KISA I/O GECİKMESİ
/// MIPS'te MMIO gecikmesi için genellikle bir senkronizasyon bariyeri kullanılır.
#[inline(always)]
pub fn io_wait() {
    sync();
}
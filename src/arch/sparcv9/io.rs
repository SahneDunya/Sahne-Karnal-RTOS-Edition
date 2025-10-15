use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;

// -----------------------------------------------------------------------------
// 1. MEMORY-MAPPED I/O (MMIO) İŞLEMLERİ
// -----------------------------------------------------------------------------

// SPARC V9'da donanım erişimi tamamen MMIO yoluyla yapılır.

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

// SPARC V9'da bellek erişimlerinin doğru sırasını sağlamak için 'membar' talimatı kullanılır.
// 'membar'ın parametreleri: #LoadLoad, #LoadStore, #StoreLoad, #StoreStore

/// Genel Bellek Bariyeri (MEMBAR #StoreLoad | #StoreStore).
/// Bu talimattan önce başlayan tüm yazma işlemlerinin, bu talimattan sonra başlayan
/// okuma ve yazma işlemlerinden önce tamamlanmasını sağlar. 
/// Genellikle MMIO yazma işlemlerinden sonra kullanılır.
#[inline(always)]
pub fn membar_store_sync() {
    unsafe {
        // SPARC assembly: 'membar #StoreLoad | #StoreStore'
        // Bu genellikle MMIO yazmalarından sonra yeterlidir.
        asm!("membar #StoreLoad | #StoreStore", options(nomem, nostack)); 
    }
}

/// Tam Bellek Bariyeri (MEMBAR #ALL).
/// Tüm bellek erişimlerinin (okuma ve yazma) tam senkronizasyonunu sağlar.
#[inline(always)]
pub fn membar_all() {
    unsafe {
        // SPARC assembly: 'membar #LoadLoad | #LoadStore | #StoreLoad | #StoreStore'
        // Veya kısaca: 'membar #Sync'
        asm!("membar #Sync", options(nomem, nostack));
    }
}

// -----------------------------------------------------------------------------
// 3. YARDIMCI FONKSİYONLAR
// -----------------------------------------------------------------------------

/// KISA I/O GECİKMESİ
/// SPARC V9'da MMIO gecikmesi için genellikle bir bellek bariyeri (`membar`) kullanılır.
#[inline(always)]
pub fn io_wait() {
    membar_store_sync();
}
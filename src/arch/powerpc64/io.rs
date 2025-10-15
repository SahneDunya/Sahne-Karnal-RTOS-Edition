use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;

// -----------------------------------------------------------------------------
// 1. MEMORY-MAPPED I/O (MMIO) İŞLEMLERİ
// -----------------------------------------------------------------------------

// PowerPC'de donanım erişimi tamamen MMIO yoluyla yapılır.

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
// 2. BELLEK VE TALİMAT BARİYERLERİ (SENKRONİZASYON)
// -----------------------------------------------------------------------------

// PowerPC'de komutların ve veri erişimlerinin sıralamasını garanti etmek için bariyerler kritiktir.

/// Bellek Senkronizasyon Bariyeri (SYNC).
/// Bu talimattan önce başlayan tüm bellek erişimlerinin, bu talimattan sonra başlayan 
/// herhangi bir erişimden önce tamamlanmasını sağlar. 
/// Genellikle MMIO yazma işlemlerinden sonra kullanılır.
#[inline(always)]
pub fn sync() {
    unsafe {
        // PowerPC assembly: 'sync'
        asm!("sync", options(nomem, nostack));
    }
}

/// Talimat Senkronizasyon Bariyeri (ISYNC - Instruction Synchronization).
/// Boru hattını boşaltır, böylece bu talimattan sonra gelen tüm talimatlar yeni 
/// geçerli durum ile birlikte getirilir ve yürütülür (Örn: MMU/Sayfa Tablosu değişiklikleri).
/// Genellikle sistem yazmaçlarına (MSR, SPR) yazdıktan sonra kullanılır.
#[inline(always)]
pub fn isync() {
    unsafe {
        // PowerPC assembly: 'isync'
        asm!("isync", options(nomem, nostack));
    }
}

// -----------------------------------------------------------------------------
// 3. YARDIMCI FONKSİYONLAR
// -----------------------------------------------------------------------------

/// KISA I/O GECİKMESİ
/// PowerPC'de MMIO gecikmesi için genellikle bir Veri Bariyeri (SYNC) kullanılır.
#[inline(always)]
pub fn io_wait() {
    sync();
}
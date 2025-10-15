use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;

// -----------------------------------------------------------------------------
// 1. MEMORY-MAPPED I/O (MMIO) İŞLEMLERİ
// -----------------------------------------------------------------------------

// ARMv9'da donanım erişimi tamamen MMIO yoluyla yapılır.

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

// ARM mimarisinde, komutların ve veri erişimlerinin sıralamasını garanti etmek 
// için bariyerler kritik öneme sahiptir.

/// Veri Senkronizasyon Bariyeri (DSB - Data Synchronization Barrier).
/// Bu talimattan önce başlayan tüm bellek erişimlerinin, bu talimattan sonra başlayan 
/// herhangi bir talimatın yürütülmesine devam etmeden önce tamamlanmasını sağlar.
/// Genellikle MMIO yazma işlemlerinden sonra kullanılır.
#[inline(always)]
pub fn dsb() {
    unsafe {
        // 'sy' (System) tüm cihazlar ve bellek üzerinde tam bariyer anlamına gelir.
        asm!("dsb sy", options(nomem, nostack));
    }
}

/// Talimat Senkronizasyon Bariyeri (ISB - Instruction Synchronization Barrier).
/// Boru hattını boşaltır, böylece bu talimattan sonra gelen tüm talimatlar yeni 
/// geçerli durum (örneğin, değiştirilen sayfa tablosu, değiştirilen sistem yazmaçları) 
/// ile birlikte getirilir ve yürütülür.
/// Genellikle sistem yazmaçlarına (System Registers) yazdıktan sonra kullanılır.
#[inline(always)]
pub fn isb() {
    unsafe {
        asm!("isb", options(nomem, nostack));
    }
}

// -----------------------------------------------------------------------------
// 3. YARDIMCI FONKSİYONLAR
// -----------------------------------------------------------------------------

/// KISA I/O GECİKMESİ
/// ARM'da MMIO gecikmesi için genellikle bir döngü veya basit bir MMIO işlemi kullanılır.
#[inline(always)]
pub fn io_wait() {
    // DSB kullanmak genellikle en iyi uygulamadır, ancak bazen basit bir okuma 
    // gecikme için yeterli olabilir. Burada DSB kullanmayı tercih ediyoruz.
    dsb();
}
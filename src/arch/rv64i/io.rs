use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;

// -----------------------------------------------------------------------------
// 1. MEMORY-MAPPED I/O (MMIO) İŞLEMLERİ
// -----------------------------------------------------------------------------

// RISC-V'de donanım erişimi tamamen MMIO yoluyla yapılır.

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

// RISC-V'de bellek erişimlerinin doğru sırasını sağlamak için 'fence' talimatı kullanılır.
// 'fence' talimatının parametreleri: pred (önceki erişimler), succ (sonraki erişimler)
// rw = read/write (okuma/yazma), i = instruction (talimat)

/// Genel Bellek Bariyeri (FENCE R, W | FENCE).
/// Bu talimattan önce başlayan tüm okuma ve yazma işlemlerinin, bu talimattan sonra başlayan
/// okuma ve yazma işlemlerinden önce tamamlanmasını sağlar. 
/// Genellikle MMIO yazma işlemlerinden sonra kullanılır.
#[inline(always)]
pub fn fence() {
    unsafe {
        // RISC-V assembly: 'fence rw, rw'
        // 'rw' (read/write) önceki tüm okuma/yazmaların, sonraki tüm okuma/yazmalardan önce bitmesini garanti eder.
        asm!("fence rw, rw", options(nomem, nostack)); 
    }
}

/// Talimat Senkronizasyon Bariyeri (FENCE.I).
/// Komut önbelleğini temizler ve talimat akışının yeni geçerli durum ile devam etmesini sağlar.
/// Genellikle Sayfa Tablosu (MMU) veya Vektör Tablosu (STVEC) gibi yazmaçları değiştirdikten sonra kullanılır.
#[inline(always)]
pub fn fence_i() {
    unsafe {
        // RISC-V assembly: 'fence.i'
        asm!("fence.i", options(nomem, nostack));
    }
}

// -----------------------------------------------------------------------------
// 3. YARDIMCI FONKSİYONLAR
// -----------------------------------------------------------------------------

/// KISA I/O GECİKMESİ
/// RISC-V'de MMIO gecikmesi için genellikle bir bellek bariyeri (`fence`) kullanılır.
#[inline(always)]
pub fn io_wait() {
    fence();
}
use core::arch::asm;
use core::ptr::{read_volatile, write_volatile};

// -----------------------------------------------------------------------------
// 1. PORT I/O (IN/OUT) İŞLEMLERİ
// -----------------------------------------------------------------------------

// --- YAZMA İŞLEMLERİ (OUT) ---

/// Belirtilen I/O portuna bir bayt (u8) yazar. (OUTB)
///
/// # Güvenlik Notu
/// Bu fonksiyon doğrudan donanım yazmaçlarına erişir ve yetkilendirme (I/O İzin Seviyesi)
/// gerektirir. Sadece güvenli (unsafe) bağlamda kullanılmalıdır.
#[inline(always)]
pub unsafe fn port_outb(port: u16, data: u8) {
    // 'outb %al, %dx'
    // 'nomem' ve 'nostack' seçenekleri, derleyiciye belleğe veya yığına erişilmediğini söyler.
    asm!("outb %al, %dx", in("dx") port, in("al") data, options(nomem, nostack));
}

/// Belirtilen I/O portuna bir kelime (u16) yazar. (OUTW)
#[inline(always)]
pub unsafe fn port_outw(port: u16, data: u16) {
    // 'outw %ax, %dx'
    asm!("outw %ax, %dx", in("dx") port, in("ax") data, options(nomem, nostack));
}

/// Belirtilen I/O portuna bir çift kelime (u32) yazar. (OUTL)
#[inline(always)]
pub unsafe fn port_outl(port: u16, data: u32) {
    // 'outl %eax, %dx'
    asm!("outl %eax, %dx", in("dx") port, in("eax") data, options(nomem, nostack));
}


// --- OKUMA İŞLEMLERİ (IN) ---

/// Belirtilen I/O portundan bir bayt (u8) okur. (INB)
///
/// # Güvenlik Notu
/// Bu fonksiyon doğrudan donanım yazmaçlarına erişir ve yetkilendirme gerektirir.
#[inline(always)]
pub unsafe fn port_inb(port: u16) -> u8 {
    let data: u8;
    // 'inb %dx, %al'
    asm!("inb %dx, %al", out("al") data, in("dx") port, options(nomem, nostack));
    data
}

/// Belirtilen I/O portundan bir kelime (u16) okur. (INW)
#[inline(always)]
pub unsafe fn port_inw(port: u16) -> u16 {
    let data: u16;
    // 'inw %dx, %ax'
    asm!("inw %dx, %ax", out("ax") data, in("dx") port, options(nomem, nostack));
    data
}

/// Belirtilen I/O portundan bir çift kelime (u32) okur. (INL)
#[inline(always)]
pub unsafe fn port_inl(port: u16) -> u32 {
    let data: u32;
    // 'inl %dx, %eax'
    asm!("inl %dx, %eax", out("eax") data, in("dx") port, options(nomem, nostack));
    data
}

// -----------------------------------------------------------------------------
// 2. MEMORY-MAPPED I/O (MMIO) İŞLEMLERİ
// -----------------------------------------------------------------------------

// MMIO, temel olarak bir bellek adresine volatile (uçucu) okuma/yazma işlemidir.
// Derleyicinin optimizasyon yapmasını engeller.

/// Belirtilen bellek adresinden (MMIO) bir u8 okur.
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
// 3. YARDIMCI FONKSİYONLAR
// -----------------------------------------------------------------------------

/// KISA I/O GECİKMESİ (PIC komutları arasında gereklidir)
/// Bir I/O işleminden sonra kısa bir gecikme ekler.
/// Genellikle port 0x80'e yazarak yapılır.
#[inline(always)]
pub fn io_wait() {
    // Port 0x80 genellikle bir 'checkpoint' portu olarak kullanılır.
    // Sadece yazma işlemi bir miktar gecikme sağlar.
    unsafe { port_outb(0x80, 0) };
}
// src/arch/openrisc64/platformmod.rs
// OpenRISC 64 (OR64) mimarisine özgü platform başlatma ve temel G/Ç işlevleri.

#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr;
use crate::serial_println;

/// Bu modül, diğer mimariye özgü modüller tarafından kullanılacak temel G/Ç
/// ve kontrol işlevlerini içerir.
pub mod io {
    use core::arch::asm;
    use core::ptr;

    // -------------------------------------------------------------------------
    // MMIO (Memory-Mapped I/O) Fonksiyonları
    // -------------------------------------------------------------------------

    /// Verilen bellek adresinden 8 bit (byte) okur (Volatile).
    #[inline(always)]
    pub unsafe fn read_mmio_8(addr: usize) -> u8 {
        ptr::read_volatile(addr as *const u8)
    }

    /// Verilen bellek adresine 8 bit (byte) yazar (Volatile).
    #[inline(always)]
    pub unsafe fn write_mmio_8(addr: usize, value: u8) {
        ptr::write_volatile(addr as *mut u8, value)
    }
    
    // NOT: 16, 32 ve 64 bit MMIO okuma/yazma fonksiyonları gerektiğinde eklenebilir.

    // -------------------------------------------------------------------------
    // Senkronizasyon (Bariyer) Fonksiyonları
    // -------------------------------------------------------------------------

    /// Talimat senkronizasyonu bariyeri.
    /// OpenRISC'te 'l.isync' kullanılır.
    #[inline(always)]
    pub unsafe fn isync() {
        // Assembly: l.isync
        asm!("l.isync", options(nomem, nostack)); 
    }

    /// Veri senkronizasyonu bariyeri.
    /// OpenRISC'te 'l.msync' (Memory Sync) veya özel bir bariyer kullanılır.
    #[inline(always)]
    pub unsafe fn dsync() {
        // Assembly: l.msync (Temsili olarak)
        asm!("l.msync", options(nomem, nostack)); 
    }
    
    /// Tam bellek ve talimat bariyeri.
    #[inline(always)]
    pub unsafe fn membar_all() {
        dsync();
        isync();
    }

    // -------------------------------------------------------------------------
    // Kontrol Fonksiyonları
    // -------------------------------------------------------------------------

    /// İşlemciyi bir olay gelene kadar düşük güç modunda bekletir (IDLE).
    #[inline(always)]
    pub unsafe fn idle() {
        // Assembly: l.nop (OpenRISC'te düşük güç bekleme genellikle NOP döngüsü ile yapılır)
        asm!("l.nop", options(nomem, nostack, preserves_flags)); 
    }
    
    // -------------------------------------------------------------------------
    // SPR (Special Purpose Register) Erişim Fonksiyonları
    // -------------------------------------------------------------------------

    // Örnek SPR Numaraları (OpenRISC Mimarisine Göre Temsili)
    pub const SPR_SR: u32 = 0x11;    // Status Register (Durum Yazmacı)
    pub const SPR_EPC: u32 = 0x16;   // Exception Program Counter
    pub const SPR_ESR: u32 = 0x17;   // Exception Status Register

    /// Belirtilen SPR yazmacını okur.
    ///
    /// # Parametreler:
    /// * `reg_num`: SPR yazmacının numarası.
    #[inline(always)]
    pub unsafe fn read_spr(reg_num: u32) -> u64 {
        let value: u64;
        // Assembly: l.mfspr rd, r0, spr_num (Move From SPR)
        // Rust'ta, `r0` yerine bir operand olarak 0 yazmacını kullanmak gerekir.
        asm!("l.mfspr {0}, $r0, {1}", 
             out(reg) value, 
             in(reg) reg_num, 
             options(nomem, nostack));
        value
    }

    /// Belirtilen SPR yazmacına yazar.
    #[inline(always)]
    pub unsafe fn write_spr(reg_num: u32, value: u64) {
        // Assembly: l.mtspr r0, rs, spr_num (Move To SPR)
        // Rust'ta, `r0` yerine bir operand olarak 0 yazmacını kullanmak gerekir.
        asm!("l.mtspr $r0, {0}, {1}", 
             in(reg) value, 
             in(reg) reg_num, 
             options(nomem, nostack));
    }
    
    /// Kesmeleri devre dışı bırakır (SR yazmacı üzerinden).
    #[inline(always)]
    pub unsafe fn disable_interrupts() {
        let status = read_spr(SPR_SR);
        // EE (Exception Enable) bitini (genellikle 1. bit) temizle
        const SR_EE: u64 = 1 << 0;
        write_spr(SPR_SR, status & !SR_EE);
    }
    
    /// Kesmeleri etkinleştirir (SR yazmacı üzerinden).
    #[inline(always)]
    pub unsafe fn enable_interrupts() {
        let status = read_spr(SPR_SR);
        // EE (Exception Enable) bitini ayarla
        const SR_EE: u64 = 1 << 0;
        write_spr(SPR_SR, status | SR_EE);
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA FONKSİYONU
// -----------------------------------------------------------------------------

/// OpenRISC 64 mimarisine özgü temel donanım yapılandırmalarını başlatır.
/// Bu fonksiyon `main.rs`'ten çekirdek başlangıcında çağrılmalıdır.
pub fn platform_init() {
    serial_println!("[OR64] Mimariye Özgü Başlatma Başlatılıyor...");
    
    // 1. Kesmeleri devre dışı bırak (Güvenlik için)
    unsafe {
        io::disable_interrupts();
    }
    serial_println!("[OR64] Kesmeler devre dışı bırakıldı.");

    // 2. SPR yazmaçlarının başlangıç durumunu kontrol etme
    let current_status = unsafe { io::read_spr(io::SPR_SR) };
    serial_println!("[OR64] Başlangıç SR Değeri: {:#x}", current_status);

    // 3. Geçerli durumun senkronize edilmesi
    unsafe {
        io::membar_all();
    }

    // 4. Diğer alt sistemleri başlat (MMU, İstisnalar, Zamanlayıcı, vb.)
    
    serial_println!("[OR64] Temel Platform Hazır.");
}
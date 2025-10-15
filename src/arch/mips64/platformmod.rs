// src/arch/mips64/platformmod.rs
// MIPS64 mimarisine özgü platform başlatma ve temel G/Ç işlevleri.

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

    /// Tam bir bellek ve talimat senkronizasyon bariyeri sağlar.
    #[inline(always)]
    pub unsafe fn sync() {
        // Assembly: sync
        asm!("sync", options(nomem, nostack)); 
    }

    /// Talimat akışını duraklatır (kesme gelene kadar bekler).
    #[inline(always)]
    pub unsafe fn wait() {
        // Assembly: wait
        asm!("wait", options(nomem, nostack, preserves_flags)); 
    }
    
    // -------------------------------------------------------------------------
    // CP0 (Co-Processor 0) Kontrol Yazmaçları Erişim Fonksiyonları
    // -------------------------------------------------------------------------

    // Örnek CP0 Yazmaç Numaraları ve Seçici Alanları
    pub const CP0_STATUS: u32 = 12;   // Durum Yazmacı (Interrupt Enable, vb. içerir)
    pub const CP0_CAUSE: u32 = 13;    // Neden Yazmacı
    pub const CP0_EPC: u32 = 14;      // İstisna Program Sayacı

    /// Belirtilen CP0 yazmacını okur.
    ///
    /// # Parametreler:
    /// * `reg_num`: CP0 yazmacının numarası (0-31).
    /// * `sel`: Yazmacın seçici alanı (genellikle 0).
    #[inline(always)]
    pub unsafe fn read_cp0(reg_num: u32, sel: u32) -> u64 {
        let value: u64;
        // Assembly: mfc0 rt, cp0_reg, sel (Move From Co-Processor 0)
        asm!("mfc0 {0}, ${1}, {2}", 
             out(reg) value, 
             const reg_num, 
             const sel, 
             options(nomem, nostack));
        value
    }

    /// Belirtilen CP0 yazmacına yazar.
    #[inline(always)]
    pub unsafe fn write_cp0(reg_num: u32, sel: u32, value: u64) {
        // Assembly: mtc0 rs, cp0_reg, sel (Move To Co-Processor 0)
        asm!("mtc0 {0}, ${1}, {2}", 
             in(reg) value, 
             const reg_num, 
             const sel, 
             options(nomem, nostack));
    }
    
    /// Kesmeleri devre dışı bırakır.
    #[inline(always)]
    pub unsafe fn disable_interrupts() {
        let status = read_cp0(CP0_STATUS, 0);
        // IE (Interrupt Enable) bitini (genellikle 1. bit) temizle
        const STATUS_IE: u64 = 0x1;
        write_cp0(CP0_STATUS, 0, status & !STATUS_IE);
    }
    
    /// Kesmeleri etkinleştirir.
    #[inline(always)]
    pub unsafe fn enable_interrupts() {
        let status = read_cp0(CP0_STATUS, 0);
        // IE (Interrupt Enable) bitini ayarla
        const STATUS_IE: u64 = 0x1;
        write_cp0(CP0_STATUS, 0, status | STATUS_IE);
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA FONKSİYONU
// -----------------------------------------------------------------------------

/// MIPS64 mimarisine özgü temel donanım yapılandırmalarını başlatır.
/// Bu fonksiyon `main.rs`'ten çekirdek başlangıcında çağrılmalıdır.
pub fn platform_init() {
    serial_println!("[MIPS64] Mimariye Özgü Başlatma Başlatılıyor...");
    
    // 1. Kesmeleri devre dışı bırak (Güvenlik için)
    unsafe {
        io::disable_interrupts();
    }
    serial_println!("[MIPS64] Kesmeler devre dışı bırakıldı.");

    // 2. CP0 yazmaçlarının başlangıç durumunu kontrol etme
    let current_status = unsafe { io::read_cp0(io::CP0_STATUS, 0) };
    serial_println!("[MIPS64] Başlangıç STATUS Değeri: {:#x}", current_status);

    // 3. Temsili MMU/Önbellek ayarları (gelişmiş kodda buraya eklenir)
    // MMU'nun başlatılması ayrı `mmu.rs` modülünde yapılır.

    // 4. Talimat senkronizasyonu
    unsafe {
        io::sync();
    }

    serial_println!("[MIPS64] Temel Platform Hazır.");
}
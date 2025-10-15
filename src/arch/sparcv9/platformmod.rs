// src/arch/sparcv9/platformmod.rs
// SPARC V9 (UltraSPARC) mimarisine özgü platform başlatma ve temel G/Ç işlevleri.

#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr;
use crate::serial_println;

// -----------------------------------------------------------------------------
// ADRES UZAYI TANITICILARI (ASI)
// -----------------------------------------------------------------------------

// UltraSPARC'ta ASI'lar, Load/Store talimatlarının hangi adres uzayına (donanım/kontrol yazmaçları)
// erişeceğini belirler.
// Temsili ASI değerleri:
const ASI_PRIMARY: u8 = 0x8;       // Normal Veri Erişimleri
const ASI_P_CONTEXT: u8 = 0x48;    // Primary Context (MMU/TLB kontrolü için yaygın)
const ASI_N_CONTEXT: u8 = 0x49;    // Nucleus Context
const ASI_MMU_REGS: u8 = 0x40;     // Temsili MMU Kontrol Yazmaçları ASI'sı

/// Bu modül, diğer mimariye özgü modüller tarafından kullanılacak temel G/Ç
/// ve kontrol işlevlerini içerir.
pub mod io {
    use core::arch::asm;
    use core::ptr;
    use super::ASI_PRIMARY;

    // -------------------------------------------------------------------------
    // MMIO (Memory-Mapped I/O) Fonksiyonları - ASI Kullanarak
    // -------------------------------------------------------------------------

    /// Verilen bellek adresinden ve ASI'dan 8 bit (byte) okur.
    ///
    /// # Not: SPARC'ta genellikle 32/64-bit erişim kullanılır. Bu, 8-bit'lik
    /// bir MMIO cihazı için temsilidir (lduba - Load Unsigned Byte using Alternate ASI).
    #[inline(always)]
    pub unsafe fn read_mmio_8(addr: usize, asi: u8) -> u8 {
        let value: u8;
        // Assembly: lduw [rs1 + rs2] asi, rd
        // lduba: Load Unsigned Byte from Alternate space
        asm!("lduba [{addr}] {asi}, {value}", 
             addr = in(reg) addr, 
             asi = in(reg) asi, 
             value = out(reg) value, 
             options(nomem, nostack));
        value
    }

    /// Verilen bellek adresine ve ASI'ya 8 bit (byte) yazar.
    ///
    /// # Not: stba (Store Byte to Alternate space)
    #[inline(always)]
    pub unsafe fn write_mmio_8(addr: usize, asi: u8, value: u8) {
        // Assembly: stba rs2, [rs1 + rs3] asi
        asm!("stba {value}, [{addr}] {asi}", 
             addr = in(reg) addr, 
             asi = in(reg) asi, 
             value = in(reg) value, 
             options(nomem, nostack));
    }
    
    // Basit MMIO için birincil veri ASI'sini kullanan kısayol (ptr::read_volatile gibi)
    #[inline(always)]
    pub unsafe fn read_mmio_64(addr: usize) -> u64 {
        let value: u64;
        // Assembly: ldx [rs1] asi, rd (Load Double word from Primary space)
        asm!("ldx [{addr}] {asi}, {value}", 
             addr = in(reg) addr, 
             asi = const ASI_PRIMARY, 
             value = out(reg) value, 
             options(nomem, nostack));
        value
    }
    
    #[inline(always)]
    pub unsafe fn write_mmio_64(addr: usize, value: u64) {
        // Assembly: stx rs2, [rs1] asi (Store Double word to Primary space)
        asm!("stx {value}, [{addr}] {asi}", 
             addr = in(reg) addr, 
             asi = const ASI_PRIMARY, 
             value = in(reg) value, 
             options(nomem, nostack));
    }


    // -------------------------------------------------------------------------
    // Senkronizasyon (Bariyer) Fonksiyonları
    // -------------------------------------------------------------------------

    /// Bellek Bariyeri (Memory Barrier - MEMBAR).
    /// UltraSPARC'ta bu, tüm veri erişimlerinin sıralanmasını sağlar.
    /// # Parametreler: maske (Sync, LoadStore, vb. birleşimi)
    #[inline(always)]
    pub unsafe fn membar_all() {
        // Temsili maske: #Sync (0xF)
        asm!("membar #Sync", options(nomem, nostack)); 
    }

    /// Kayıt Penceresi Temizleme (Flush Windows).
    /// Tüm kayıt pencerelerini ana belleğe/yığına boşaltır.
    #[inline(always)]
    pub unsafe fn flushw() {
        // Assembly: flushw
        asm!("flushw", options(nomem, nostack)); 
    }

    // -------------------------------------------------------------------------
    // Kontrol Fonksiyonları
    // -------------------------------------------------------------------------

    /// İşlemciyi bir kesme gelene kadar düşük güç modunda bekletir.
    /// UltraSPARC'ta bu, genellikle özel bir talimatla yapılır (örn: `snoop` veya `idle` benzeri bir durum).
    /// Basitlik için NOP kullanılır.
    #[inline(always)]
    pub unsafe fn idle() {
        // Assembly: nop
        asm!("nop", options(nomem, nostack, preserves_flags)); 
    }
    
    // -------------------------------------------------------------------------
    // ASR (Ancillary State Register) / Kontrol Yazmaçları Erişim Fonksiyonları
    // -------------------------------------------------------------------------

    // ASR'lar da aslında özel ASI'lar ile erişilen kontrol yazmaçlarıdır.
    // Örn: MMU yazmaçları (ASI_MMU_REGS), Process State Register (PSR)
    
    /// PSR (Process State Register) okur (Kesme etkinleştirme içerir).
    #[inline(always)]
    pub unsafe fn read_psr() -> u64 {
        let value: u64;
        // Assembly: rdpr rd, %psr (Read Processor Register)
        asm!("rdpr {0}, %psr", out(reg) value, options(nomem, nostack));
        value
    }

    /// PSR (Process State Register) yazar.
    #[inline(always)]
    pub unsafe fn write_psr(value: u64) {
        // Assembly: wrpr rs, %psr
        asm!("wrpr {0}, %psr", in(reg) value, options(nomem, nostack));
    }
    
    /// Kesmeleri devre dışı bırakır (PSR yazmacı üzerinden).
    #[inline(always)]
    pub unsafe fn disable_interrupts() {
        let psr = read_psr();
        // PEF (Processor Error Fatal) ve PIL (Processor Interrupt Level) bitlerini temizle
        // PIL: 1 (en düşük öncelik) olarak ayarlanırsa, tüm kesmeler engellenir.
        // PIL alanı: PSR[11:8]
        const PSR_PIL_MASK: u64 = 0xF << 8;
        // PIL=15: Tüm kesmeleri maskele
        const PSR_PIL_MAX: u64 = 0xF << 8; 
        
        write_psr((psr & !PSR_PIL_MASK) | PSR_PIL_MAX);
    }
    
    /// Kesmeleri etkinleştirir (PSR yazmacı üzerinden).
    #[inline(always)]
    pub unsafe fn enable_interrupts() {
        let psr = read_psr();
        // PIL=0: Kesmeleri etkinleştir (sıfırdan farklı olanlar tetiklenir)
        const PSR_PIL_MASK: u64 = 0xF << 8;
        const PSR_PIL_ZERO: u64 = 0x0 << 8;
        
        write_psr((psr & !PSR_PIL_MASK) | PSR_PIL_ZERO);
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA FONKSİYONU
// -----------------------------------------------------------------------------

/// SPARC V9 mimarisine özgü temel donanım yapılandırmalarını başlatır.
/// Bu fonksiyon `main.rs`'ten çekirdek başlangıcında çağrılmalıdır.
pub fn platform_init() {
    serial_println!("[SPARC V9] Mimariye Özgü Başlatma Başlatılıyor...");
    
    // 1. Kayıt pencerelerini temizle (Güvenli başlangıç için)
    unsafe {
        io::flushw();
    }
    
    // 2. Kesmeleri devre dışı bırak (Güvenlik için)
    unsafe {
        io::disable_interrupts();
    }
    serial_println!("[SPARC V9] Kesmeler devre dışı bırakıldı (PIL=15).");

    // 3. PSR yazmacının başlangıç durumunu kontrol etme
    let current_psr = unsafe { io::read_psr() };
    serial_println!("[SPARC V9] Başlangıç PSR Değeri: {:#x}", current_psr);

    // 4. Senkronizasyon
    unsafe {
        io::membar_all();
    }

    // 5. Diğer alt sistemleri başlat (MMU, İstisnalar, Zamanlayıcı, vb.)
    
    serial_println!("[SPARC V9] Temel Platform Hazır.");
}
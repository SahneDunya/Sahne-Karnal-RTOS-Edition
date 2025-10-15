#![allow(dead_code)]
#![allow(non_snake_case)]

use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;
use crate::serial_println;

// -----------------------------------------------------------------------------
// VİRTÜEL KESME KONTROLCÜSÜ MMIO ADRESLERİ (Temsili Adresler)
// -----------------------------------------------------------------------------

// SPARC V9 (UltraSPARC) platformlarında harici kesmeleri yöneten varsayımsal bir PIC/MPIC.
// Bu adresler donanıma göre değişir ve PROM/DTB'den alınmalıdır.
const INTR_CONTROLLER_BASE: usize = 0xF000_0000; 

// Kesme Kontrolcüsü Yazmaç Ofsetleri (Temsili)
const INTR_ENABLE: usize    = 0x0;  // Kesme Etkinleştirme (Maskeleme) Yazmacı (64-bit)
const INTR_PENDING: usize   = 0x8;  // Bekleyen Kesme (Pending) Yazmacı (64-bit)
const INTR_ACK: usize       = 0x10; // Kesme Onaylama (Acknowledge) Yazmacı
const INTR_CLEAR: usize     = 0x18; // Kesme Temizleme (Clear/EOI) Yazmacı

// -----------------------------------------------------------------------------
// MMIO VE KONTROLCÜ İŞLEMLERİ
// -----------------------------------------------------------------------------

/// Kontrolcü Yazmaçlarından 64 bitlik veri okur.
#[inline(always)]
unsafe fn intr_read_reg(offset: usize) -> u64 {
    read_volatile((INTR_CONTROLLER_BASE + offset) as *const u64)
}

/// Kontrolcü Yazmaçlarına 64 bitlik veri yazar.
#[inline(always)]
unsafe fn intr_write_reg(offset: usize, value: u64) {
    write_volatile((INTR_CONTROLLER_BASE + offset) as *mut u64, value)
}

/// Harici Kesme Kontrolcüsünü başlatır.
pub unsafe fn init_controller() {
    // 1. Tüm harici kesmeleri maskele (devre dışı bırak).
    intr_write_reg(INTR_ENABLE, 0x0000_0000_0000_0000); 
    
    // 2. Bekleyen tüm kesmeleri temizle (varsa).
    // intr_write_reg(INTR_CLEAR, 0xFFFFFFFFFFFFFFFF); 

    serial_println!("[SPARCV9] Harici Kesme Kontrolcüsü Başlatıldı.");
}

/// Bir kesme işleyicisinin çalışması bittiğinde kontrolcüye bildirim gönderir (EOI).
/// 
/// # Parametreler
/// * `irq_line`: İşlenmesi biten harici kesmenin numarası (IRQ 0-63).
pub unsafe fn send_eoi(irq_line: u32) {
    // EOI için kesmeyi temizleme yazmacına yazma
    intr_write_reg(INTR_CLEAR, 1 << irq_line);
}

/// Kontrolcüden bekleyen bir kesme olup olmadığını kontrol eder ve ID'sini alır.
/// 
/// # Geri Dönüş
/// Bekleyen kesmenin IRQ numarası (1-63) veya bekleyen kesme yoksa 0.
pub unsafe fn get_irq() -> u32 {
    // PIC_PENDING yazmacını oku
    let pending = intr_read_reg(INTR_PENDING);
    
    if pending == 0 {
        return 0; // Bekleyen kesme yok.
    }
    
    // En yüksek öncelikli bekleyen kesme bitini bul (En az anlamlı biti kullanalım)
    63 - pending.leading_zeros()
}


/// Belirtilen IRQ hattını maskeler (devre dışı bırakır).
pub unsafe fn mask_irq(irq_line: u32) {
    let enable_reg = intr_read_reg(INTR_ENABLE);
    // İlgili biti 0'a ayarla (0 = maskeli/devre dışı varsayımı)
    let new_enable = enable_reg & !(1 << irq_line); 
    intr_write_reg(INTR_ENABLE, new_enable);
}

/// Belirtilen IRQ hattının maskesini kaldırır (etkinleştirir).
pub unsafe fn unmask_irq(irq_line: u32) {
    let enable_reg = intr_read_reg(INTR_ENABLE);
    // İlgili biti 1'e ayarla (1 = maskesiz/etkin varsayımı)
    let new_enable = enable_reg | (1 << irq_line); 
    intr_write_reg(INTR_ENABLE, new_enable);
}

// -----------------------------------------------------------------------------
// CPU KESME KONTROLÜ (TSTATE - Trap State Register)
// -----------------------------------------------------------------------------

/// Genel kesme (IE - Interrupt Enable) bayrağını etkinleştirir.
pub fn enable_interrupts() {
    unsafe {
        // TSTATE'i oku
        let mut tstate: u64;
        asm!("rdpr %tstate, {}", out(reg) tstate); 
        
        // IE (Interrupt Enable) bitini ayarla (TSTATE bit 17)
        tstate |= 1 << 17; 
        
        // TSTATE'e yaz
        asm!("wrpr {}, %tstate", in(reg) tstate); 
    }
}

/// Genel kesme (IE) bayrağını devre dışı bırakır.
pub fn disable_interrupts() {
    unsafe {
        // TSTATE'i oku
        let mut tstate: u64;
        asm!("rdpr %tstate, {}", out(reg) tstate); 

        // IE (Interrupt Enable) bitini temizle
        tstate &= !(1 << 17); 
        
        // TSTATE'e yaz
        asm!("wrpr {}, %tstate", in(reg) tstate); 
    }
}


// -----------------------------------------------------------------------------
// KESME İŞLEME MANTIĞI ENTEGRASYONU
// -----------------------------------------------------------------------------

/// Gelen Harici Kesmeleri İşleme.
/// Bu fonksiyon, `src/arch/sparcv9/exception.rs` dosyasındaki `handle_interrupt`
/// tarafından çağrılmalıdır.
pub fn handle_external_interrupts() {
    unsafe {
        // Hangi kesmenin beklediğini kontrolcüden öğren
        let irq_line = get_irq();

        if irq_line > 0 {
            serial_println!("IRQ {} geldi.", irq_line);
            
            // 1. IRQ'yu işle (Uygun sürücüyü çağır)
            arch::sparcv9::driver::handle_irq(irq_line);
            
            // 2. EOI gönder
            send_eoi(irq_line);
        }
    }
}
#![allow(dead_code)]
#![allow(non_snake_case)]

use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;
use crate::serial_println;

// -----------------------------------------------------------------------------
// VİRTÜEL PIC MMIO ADRESLERİ (Temsili Adresler)
// -----------------------------------------------------------------------------

// OpenRISC platformlarında harici kesmeleri yöneten varsayımsal bir PIC.
const PIC_BASE: usize = 0x9000_1000; 

// PIC Yazmaç Ofsetleri
const PIC_ENABLE: usize    = 0x0;  // Kesme Etkinleştirme (Maskeleme) Yazmacı (32-bit)
const PIC_PENDING: usize   = 0x4;  // Bekleyen Kesme (Pending) Yazmacı (32-bit)
const PIC_EOI: usize       = 0x8;  // Kesme Sonu (EOI) Yazmacı

// -----------------------------------------------------------------------------
// MMIO VE PIC KONTROLÜ
// -----------------------------------------------------------------------------

/// PIC Yazmaçlarından 32 bitlik veri okur.
#[inline(always)]
unsafe fn pic_read_reg(offset: usize) -> u32 {
    read_volatile((PIC_BASE + offset) as *const u32)
}

/// PIC Yazmaçlarına 32 bitlik veri yazar.
#[inline(always)]
unsafe fn pic_write_reg(offset: usize, value: u32) {
    write_volatile((PIC_BASE + offset) as *mut u32, value)
}

/// PIC'i başlatır.
pub unsafe fn init_pic() {
    // 1. Tüm harici kesmeleri maskele (devre dışı bırak).
    pic_write_reg(PIC_ENABLE, 0x0000_0000); // 0 = maskeli/devre dışı varsayımı
    
    // 2. Bekleyen tüm kesmeleri temizle.
    // (PIC mimarisine bağlıdır, bazı PIC'ler sadece okuma ile temizlenir)
    // Eğer PIC_EOI yazmaya izin veriyorsa:
    // pic_write_reg(PIC_EOI, 0x0000_0000); 

    serial_println!("[OR64] PIC Başlatıldı. Tüm harici kesmeler maskelendi.");
}

/// Bir kesme işleyicisinin çalışması bittiğinde PIC'e bildirim gönderir (EOI).
/// 
/// # Parametreler
/// * `irq_line`: İşlenmesi biten harici kesmenin numarası (IRQ 0-31).
pub unsafe fn send_eoi(irq_line: u32) {
    // PIC'e kesmenin bittiğini yazma (IRQ numarasını EOI yazmacına yazma)
    pic_write_reg(PIC_EOI, irq_line);
}

/// PIC'ten bekleyen bir kesme olup olmadığını kontrol eder.
/// 
/// # Geri Dönüş
/// Bekleyen kesmenin IRQ numarası (1-31) veya bekleyen kesme yoksa 0.
pub unsafe fn get_irq() -> u32 {
    let pending = pic_read_reg(PIC_PENDING);
    
    if pending == 0 {
        return 0; // Bekleyen kesme yok.
    }
    
    // En yüksek öncelikli bekleyen kesme bitini bul (En az anlamlı biti kullanalım)
    31 - pending.leading_zeros()
}


/// Belirtilen IRQ hattını maskeler (devre dışı bırakır).
pub unsafe fn mask_irq(irq_line: u32) {
    let enable_reg = pic_read_reg(PIC_ENABLE);
    // İlgili biti 0'a ayarla (0 = maskeli/devre dışı varsayımı)
    let new_enable = enable_reg & !(1 << irq_line); 
    pic_write_reg(PIC_ENABLE, new_enable);
}

/// Belirtilen IRQ hattının maskesini kaldırır (etkinleştirir).
pub unsafe fn unmask_irq(irq_line: u32) {
    let enable_reg = pic_read_reg(PIC_ENABLE);
    // İlgili biti 1'e ayarla (1 = maskesiz/etkin varsayımı)
    let new_enable = enable_reg | (1 << irq_line); 
    pic_write_reg(PIC_ENABLE, new_enable);
}

// -----------------------------------------------------------------------------
// CPU KESME KONTROLÜ (SRR - Supervisor Register Register)
// -----------------------------------------------------------------------------

/// Genel kesme (IE - Interrupt Enable) bayrağını etkinleştirir.
pub fn enable_interrupts() {
    unsafe {
        // l.mfspr r0, r0, 17 (r0'a SRR oku)
        let mut srr: u64;
        // Temsili olarak asm makrosu kullanıyoruz.
        // Gerçek OR64 kodu: asm!("l.mfspr {0}, r0, 17", out(reg) srr);
        // Burada sadece bayrağı kuruyoruz:
        asm!("", out(reg) srr); // Dummy read
        
        srr |= 1; // EE (Exception Enable) bitini ayarla (SRR bit 0)
        
        // Gerçek OR64 kodu: asm!("l.mtspr r0, {0}, 17", in(reg) srr);
        // Burada sadece bayrağı kuruyoruz:
        asm!("", in(reg) srr); // Dummy write
    }
}

/// Genel kesme (IE) bayrağını devre dışı bırakır.
pub fn disable_interrupts() {
    unsafe {
        let mut srr: u64;
        asm!("", out(reg) srr); // Dummy read

        srr &= !1; // EE (Exception Enable) bitini temizle
        
        asm!("", in(reg) srr); // Dummy write
    }
}


// -----------------------------------------------------------------------------
// KESME İŞLEME MANTIĞI ENTEGRASYONU
// -----------------------------------------------------------------------------

/// Gelen Harici Kesmeleri İşleme.
/// Bu fonksiyon, `src/arch/openrisc64/exception.rs` dosyasındaki `handle_interrupt`
/// tarafından çağrılmalıdır.
pub fn handle_external_interrupts() {
    unsafe {
        // Hangi kesmenin beklediğini PIC'ten öğren
        let irq_line = get_irq();

        if irq_line > 0 {
            serial_println!("IRQ {} geldi.", irq_line);
            
            // 1. IRQ'yu işle (Uygun sürücüyü çağır)
            arch::openrisc64::driver::handle_irq(irq_line);
            
            // 2. EOI gönder
            send_eoi(irq_line);
        }
    }
}
#![allow(dead_code)]
#![allow(non_snake_case)]

use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;
use crate::serial_println;

// -----------------------------------------------------------------------------
// VİRTÜEL MPIC MMIO ADRESLERİ (Temsili Adresler)
// -----------------------------------------------------------------------------

// MPIC (Multi-Processor Interrupt Controller) temel adresleri.
// Bu adresler donanıma göre değişir ve DTB'den alınmalıdır.
const MPIC_BASE: usize = 0x8000_0000; 

// MPIC Yazmaç Ofsetleri
// Temsili yazmaçlar:
const MPIC_GLOBAL_ENABLE: usize = 0x1000; // Global Kesme Etkinleştirme
const MPIC_IACK: usize          = 0x1040; // Interrupt Acknowledge Register
const MPIC_EOI: usize           = 0x1050; // End of Interrupt Register
const MPIC_IRQ_MAP_BASE: usize  = 0x2000; // Harici IRQ'ları yönlendirme/maskeleme başlangıcı

// -----------------------------------------------------------------------------
// MMIO VE PIC KONTROLÜ
// -----------------------------------------------------------------------------

/// MPIC Yazmaçlarından 32 bitlik veri okur.
#[inline(always)]
unsafe fn mpic_read_reg(offset: usize) -> u32 {
    read_volatile((MPIC_BASE + offset) as *const u32)
}

/// MPIC Yazmaçlarına 32 bitlik veri yazar.
#[inline(always)]
unsafe fn mpic_write_reg(offset: usize, value: u32) {
    write_volatile((MPIC_BASE + offset) as *mut u32, value)
}

/// MPIC'i başlatır.
pub unsafe fn init_pic() {
    // 1. Tüm harici kesmeleri maskele (devre dışı bırak).
    // Basit bir örnek olarak, 32 kesme hattının hepsini devre dışı bırakıyoruz.
    for irq in 0..32 {
        // Her IRQ hattının kendine ait bir maskeleme yazmacı olduğunu varsayalım.
        // Burası donanıma göre değişir.
        // Temsili: mpic_write_reg(MPIC_IRQ_MAP_BASE + (irq * 4), MASK_BIT);
    }
    
    // 2. MPIC'in genel etkinleştirme yazmacını ayarla (tüm CPU'lar için)
    mpic_write_reg(MPIC_GLOBAL_ENABLE, 0x0000_0001); // Örn: Enable bitini set et
    
    serial_println!("[PPC64] MPIC Başlatıldı.");
}

/// Kesme işleyicisinin çalışması bittiğinde PIC'e bildirim gönderir (EOI).
/// 
/// # Parametreler
/// * `irq_id`: İşlenmesi biten harici kesmenin numarası.
pub unsafe fn send_eoi(_irq_id: u32) {
    // PPC'de EOI genellikle sadece 0 yazarak yapılır, çünkü IACK yazmacını okuyarak 
    // kesmeyi zaten kabul etmiş oluruz.
    // EOI yazmacına 0 yazma (veya IACK'ten okunan değeri yazma)
    mpic_write_reg(MPIC_EOI, 0); 
}

/// CPU tarafından işlenecek bekleyen kesmenin ID'sini alır.
/// GIC'ye kesmeyi aldığımızı bildirir (Acknowledge).
/// 
/// # Geri Dönüş
/// Bekleyen kesmenin IRQ numarası veya bekleyen kesme yoksa 0 (veya 0xFFFFFFFF).
pub unsafe fn get_irq() -> u32 {
    // IACK (Interrupt Acknowledge Register) okuma
    // Bu okuma, aynı zamanda PIC'e kesmeyi aldığımızı da bildirir (Acknowledge).
    let iack = mpic_read_reg(MPIC_IACK);
    
    // IRQ numarasının alt bitlerde olduğunu varsayalım.
    iack & 0xFF 
}


/// Belirtilen IRQ hattını maskeler (devre dışı bırakır).
pub unsafe fn mask_irq(irq_line: u32) {
    // IRQ'yu maskeleme yazmacına erişim (IRQ'nun kendi MMIO adresi)
    // Temsili: mpic_write_reg(MPIC_IRQ_MAP_BASE + (irq_line * 4), MASK_VALUE);
}

/// Belirtilen IRQ hattının maskesini kaldırır (etkinleştirir).
pub unsafe fn unmask_irq(irq_line: u32) {
    // IRQ'yu etkinleştirme yazmacına erişim
    // Temsili: mpic_write_reg(MPIC_IRQ_MAP_BASE + (irq_line * 4), UNMASK_VALUE);
}

// -----------------------------------------------------------------------------
// CPU KESME KONTROLÜ (MSR - Machine State Register)
// -----------------------------------------------------------------------------

/// Genel kesme (EE - External Interrupt Enable) bayrağını etkinleştirir.
pub fn enable_interrupts() {
    unsafe {
        let mut msr: u64;
        // MSR'ı oku
        asm!("mfsrr1 {}", out(reg) msr, options(nomem, nostack)); 
        
        // EE (External Interrupt Enable) bitini ayarla (MSR bit 16)
        msr |= 1 << 16; 
        
        // MSR'a yaz
        asm!("mtsrr1 {}", in(reg) msr, options(nomem, nostack)); 
    }
}

/// Genel kesme (EE) bayrağını devre dışı bırakır.
pub fn disable_interrupts() {
    unsafe {
        let mut msr: u64;
        // MSR'ı oku
        asm!("mfsrr1 {}", out(reg) msr, options(nomem, nostack)); 

        // EE (External Interrupt Enable) bitini temizle
        msr &= !(1 << 16); 
        
        // MSR'a yaz
        asm!("mtsrr1 {}", in(reg) msr, options(nomem, nostack)); 
    }
}


// -----------------------------------------------------------------------------
// KESME İŞLEME MANTIĞI ENTEGRASYONU
// -----------------------------------------------------------------------------

/// Gelen Harici Kesmeleri İşleme.
/// Bu fonksiyon, `src/arch/powerpc64/exception.rs` dosyasındaki `handle_interrupt`
/// tarafından çağrılmalıdır.
pub fn handle_external_interrupts() {
    unsafe {
        // 1. Kesme ID'sini al (Acknowledge)
        let irq_id = get_irq();

        if irq_id != 0 && irq_id != 0xFFFFFFFF { // 0xFFFFFFFF genellikle bekleyen kesme yok anlamına gelir
            serial_println!("IRQ {} geldi.", irq_id);
            
            // 2. IRQ'yu işle (Uygun sürücüyü çağır)
            arch::powerpc64::driver::handle_irq(irq_id);
            
            // 3. EOI gönder
            send_eoi(irq_id);
        }
    }
}
#![allow(dead_code)]

use core::ptr::{read_volatile, write_volatile};
use crate::serial_println;

// -----------------------------------------------------------------------------
// HT-PIC MMIO ADRESLERİ (Temsili Adresler)
// -----------------------------------------------------------------------------

// Bu adresler donanıma göre değişir ve DTB'den alınmalıdır. 
// Loongson/HT-PIC mimarisi varsayımıyla temsili bir temel adres kullanıyoruz.
const HT_PIC_BASE: usize = 0x1000_0000; 

// HT-PIC Yazmaç Ofsetleri
const HT_PIC_ENABLE: usize    = 0x0;  // Kesme Etkinleştirme (Maskeleme) Yazmacı
const HT_PIC_ISR: usize       = 0x4;  // Kesme Durum Yazmacı (Interrupt Status Register)
const HT_PIC_EOI: usize       = 0x8;  // Kesme Sonu (EOI) Yazmacı
const HT_PIC_ROUTE_BASE: usize = 0x10; // Kesme Yönlendirme Yazmaçları (IRQ0 için)

// -----------------------------------------------------------------------------
// MMIO VE PIC KONTROLÜ
// -----------------------------------------------------------------------------

/// HT-PIC Yazmaçlarından 32 bitlik veri okur.
#[inline(always)]
unsafe fn pic_read_reg(offset: usize) -> u32 {
    read_volatile((HT_PIC_BASE + offset) as *const u32)
}

/// HT-PIC Yazmaçlarına 32 bitlik veri yazar.
#[inline(always)]
unsafe fn pic_write_reg(offset: usize, value: u32) {
    write_volatile((HT_PIC_BASE + offset) as *mut u32, value)
}

/// HT-PIC'i başlatır.
/// 
/// Varsayım: Bu, sadece harici (donanım) kesmelerini yönetir. 
/// Yazılım ve Zamanlayıcı kesmeleri CPU'nun CSR yazmaçları ile yönetilir.
/// 
/// # Güvenlik Notu
/// Bu fonksiyon MMIO adreslerine yazar, bu yüzden güvenli değildir.
pub unsafe fn init_pic() {
    // 1. Tüm harici kesmeleri maskele (devre dışı bırak).
    // Tüm bitler 1 (maskeli/devre dışı)
    pic_write_reg(HT_PIC_ENABLE, 0xFFFFFFFF); 
    
    // 2. Temel bir yönlendirme/mod ayarı yapılması gerekiyorsa (PIC mimarisine bağlıdır).
    // Basit bir LoongArch çekirdeği için bu adım atlanabilir.
    
    serial_println!("[LA64] HT-PIC Başlatıldı. Tüm harici kesmeler maskelendi.");
}


/// Bir kesme işleyicisinin çalışması bittiğinde PIC'e bildirim gönderir (EOI).
/// 
/// # Parametreler
/// * `irq_line`: İşlenmesi biten harici kesmenin numarası (IRQ 0, 1, vb.).
pub unsafe fn send_eoi(irq_line: u32) {
    // HT-PIC'e kesmenin bittiğini yazma (IRQ numarasını EOI yazmacına yazma)
    pic_write_reg(HT_PIC_EOI, irq_line);
}


/// HT-PIC'ten bekleyen bir kesme olup olmadığını kontrol eder.
/// 
/// # Geri Dönüş
/// Bekleyen en yüksek öncelikli kesmenin IRQ numarası veya bekleyen kesme yoksa 0 (ya da özel bir değer).
pub unsafe fn get_irq() -> u32 {
    // ISR (Interrupt Status Register) yazmacını okuyarak bekleyen kesmeleri bul.
    let status = pic_read_reg(HT_PIC_ISR);
    
    if status == 0 {
        return 0; // Bekleyen kesme yok.
    }
    
    // En yüksek öncelikli bekleyen kesme bitini bul (En düşük bit öncelikli kabul edelim)
    // Rust'ın dahili fonksiyonlarını kullanarak en az anlamlı biti bulma (ctz - count trailing zeros)
    31 - status.leading_zeros()
}


/// Belirtilen IRQ hattını maskeler (devre dışı bırakır).
pub unsafe fn mask_irq(irq_line: u32) {
    let enable_reg = pic_read_reg(HT_PIC_ENABLE);
    // İlgili biti 1'e ayarla (1 = maskeli/devre dışı)
    let new_enable = enable_reg | (1 << irq_line); 
    pic_write_reg(HT_PIC_ENABLE, new_enable);
}

/// Belirtilen IRQ hattının maskesini kaldırır (etkinleştirir).
pub unsafe fn unmask_irq(irq_line: u32) {
    let enable_reg = pic_read_reg(HT_PIC_ENABLE);
    // İlgili biti 0'a ayarla (0 = maskesiz/etkin)
    let new_enable = enable_reg & !(1 << irq_line); 
    pic_write_reg(HT_PIC_ENABLE, new_enable);
}

// -----------------------------------------------------------------------------
// KESME İŞLEME MANTIĞI ENTEGRASYONU
// -----------------------------------------------------------------------------

// Bu, src/arch/loongarch64/exception.rs dosyasındaki handle_interrupt fonksiyonundan 
// çağrılmak üzere bir şablon sunar.

/// Gelen Harici Kesmeleri İşleme.
/// Bu fonksiyon, LoongArch'un genel kesme işleyicisinden (CSR.CAUSE=INT) çağrılmalıdır.
pub fn handle_external_interrupts() {
    unsafe {
        // Hangi kesmenin beklediğini PIC'ten öğren
        let irq_line = get_irq();

        if irq_line > 0 {
            serial_println!("IRQ {} geldi.", irq_line);
            
            // 1. IRQ'yu işle (Uygun sürücüyü çağır)
            arch::loongarch64::driver::handle_irq(irq_line);
            
            // 2. EOI gönder
            send_eoi(irq_line);
        }
    }
}
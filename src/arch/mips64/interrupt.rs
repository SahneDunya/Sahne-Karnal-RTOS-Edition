#![allow(dead_code)]

use core::arch::asm;
use crate::serial_println;

// -----------------------------------------------------------------------------
// CP0 YAZMAÇLARI VE KESME YÖNETİMİ
// -----------------------------------------------------------------------------

// CP0 Yazmaç Adresleri (MIPS mimarisi tarafından tanımlanmıştır)
const CP0_STATUS: u32 = 12; // Durum Yazmacı
const CP0_CAUSE: u32  = 13; // Kesme Nedeni Yazmacı

// Status Yazmacı Bitleri
const STATUS_IE: u64  = 0x0000_0001; // Interrupt Enable (Genel Kesme Etkinleştirme)
const STATUS_EXL: u64 = 0x0000_0002; // Exception Level (İstisna Seviyesinde)
const STATUS_IM_BASE: u64 = 0x0000_FF00; // Interrupt Mask (Kesme Maskesi - Bit 8-15)

// Cause Yazmacı Bitleri
const CAUSE_IP_BASE: u64 = 0x0000_FF00; // Interrupt Pending (Bekleyen Kesme - Bit 8-15)


/// CP0 Status yazmacını okur.
#[inline(always)]
unsafe fn read_status() -> u64 {
    let status: u64;
    // mfc0 rt, rd, sel (rd=12 Status, sel=0)
    asm!("mfc0 {0}, ${1}, 0", out(reg) status, const CP0_STATUS, options(nomem, nostack));
    status
}

/// CP0 Status yazmacına yazar.
#[inline(always)]
unsafe fn write_status(status: u64) {
    // mtc0 rt, rd, sel (rd=12 Status, sel=0)
    asm!("mtc0 {0}, ${1}, 0", in(reg) status, const CP0_STATUS, options(nomem, nostack));
}

/// CP0 Cause yazmacını okur.
#[inline(always)]
unsafe fn read_cause() -> u64 {
    let cause: u64;
    // mfc0 rt, rd, sel (rd=13 Cause, sel=0)
    asm!("mfc0 {0}, ${1}, 0", out(reg) cause, const CP0_CAUSE, options(nomem, nostack));
    cause
}

// -----------------------------------------------------------------------------
// KESME KONTROL API'SI
// -----------------------------------------------------------------------------

/// Genel kesme (IE - Interrupt Enable) bayrağını etkinleştirir.
pub fn enable_interrupts() {
    unsafe {
        let status = read_status();
        write_status(status | STATUS_IE);
    }
}

/// Genel kesme (IE) bayrağını devre dışı bırakır.
pub fn disable_interrupts() {
    unsafe {
        let status = read_status();
        write_status(status & !STATUS_IE);
    }
}

/// Belirtilen IRQ hattının maskesini kaldırır (etkinleştirir).
///
/// # Parametreler
/// * `irq_line`: Etkinleştirilecek IRQ numarası (0-7).
pub fn unmask_irq(irq_line: u8) {
    if irq_line > 7 {
        return; // MIPS'te IRQ'lar genellikle 0-7 ile sınırlıdır.
    }
    unsafe {
        let status = read_status();
        // IRQ maskeleme bitleri (IM) Status yazmacının 8-15. bitlerindedir.
        let irq_mask_bit = 1 << (irq_line + 8); 
        write_status(status | irq_mask_bit);
    }
}

/// Belirtilen IRQ hattını maskeler (devre dışı bırakır).
///
/// # Parametreler
/// * `irq_line`: Devre dışı bırakılacak IRQ numarası (0-7).
pub fn mask_irq(irq_line: u8) {
    if irq_line > 7 {
        return;
    }
    unsafe {
        let status = read_status();
        let irq_mask_bit = 1 << (irq_line + 8);
        write_status(status & !irq_mask_bit);
    }
}

// -----------------------------------------------------------------------------
// KESME İŞLEME MANTIĞI ENTEGRASYONU
// -----------------------------------------------------------------------------

// MIPS'te EOI (End of Interrupt) komutu PIC'teki gibi harici bir komut değildir.
// Kesmenin temizlenmesi, ya yazılım kesmeleri için Cause yazmacına yazarak
// ya da harici kesmeler için harici donanım kontrolcüsüne (varsa) yazarak yapılır.
// Zamanlayıcı kesmesi (IRQ 7), genellikle harici yazmaçlarla temizlenir.

/// Gelen MIPS kesmelerini işler ve EOI benzeri temizlik yapar.
/// Bu fonksiyon, `src/arch/mips64/exception.rs` dosyasındaki `handle_interrupt`
/// tarafından çağrılmalıdır.
pub fn handle_interrupts() {
    unsafe {
        // Hangi kesmelerin beklediğini bul
        let cause = read_cause();
        let status = read_status();
        
        // Etkin ve bekleyen kesmeleri belirle: IP & IM
        let pending_interrupts = (cause & CAUSE_IP_BASE) & (status & STATUS_IM_BASE);

        if pending_interrupts == 0 {
            // serial_println!("[MIPS64] Boş Kesme!");
            return;
        }

        // IRQ 0'dan IRQ 7'ye kadar kontrol et
        for irq_line in 0..8 {
            let irq_bit = 1 << (irq_line + 8);
            
            if (pending_interrupts & irq_bit) != 0 {
                // serial_println!("IRQ {} geldi.", irq_line);
                
                // 1. IRQ'yu işle (Uygun sürücüyü çağır)
                // handle_driver_irq(irq_line);

                // 2. Kesme Temizleme (EOI)
                // Yazılım Kesmeleri (IP0, IP1) yazarak temizlenir
                if irq_line == 0 || irq_line == 1 {
                    // Yazılım kesmesini temizle (Cause yazmacının ilgili bitini sıfırla)
                    let new_cause = cause & !(irq_bit);
                    // mtc0 rt, rd, sel (rd=13 Cause, sel=0)
                    asm!("mtc0 {0}, ${1}, 0", in(reg) new_cause, const CP0_CAUSE, options(nomem, nostack));
                }
                
                // Harici kesmeler (IP2-IP7) Harici Kontrolcü veya Zamanlayıcı yazmacı ile temizlenir.
                // Örneğin: Zamanlayıcı (IRQ 7) için:
                if irq_line == 7 {
                    // MIPS çekirdek zamanlayıcısını temizleme (Örn: CP0.COUNT ve CP0.COMPARE yazmaçları)
                    // Bu, platforma özgüdür ve burada atlanmıştır.
                }
            }
        }
    }
}


/// Başlangıç ayarları: Tüm harici kesmeleri devre dışı bırak.
pub fn init_interrupts() {
    unsafe {
        let mut status = read_status();
        
        // 1. Genel Kesme Bayrağını devre dışı bırak
        status &= !STATUS_IE; 
        
        // 2. Tüm Kesme Maskelerini (IM[0-7]) devre dışı bırak (Yazılım hariç)
        status &= !STATUS_IM_BASE;
        
        // 3. Yazılım Kesmelerini (IM[0] ve IM[1]) etkinleştir (Yazılım aracılığıyla yönetilecekler)
        status |= (1 << 8) | (1 << 9); 
        
        write_status(status);
    }
    serial_println!("[MIPS64] Kesme Kontrolü (CP0) başlatıldı.");
}
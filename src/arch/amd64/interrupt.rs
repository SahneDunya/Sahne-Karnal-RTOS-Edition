#![allow(dead_code)]

use core::arch::asm;
use crate::serial_println;

// -----------------------------------------------------------------------------
// PORT I/O İŞLEMLERİ (I/O Portları aracılığıyla PIC kontrolü)
// -----------------------------------------------------------------------------

/// Belirtilen I/O portuna bir bayt (u8) yazar.
#[inline]
unsafe fn port_out(port: u16, data: u8) {
    // 'out' assembly komutu
    asm!("outb %al, %dx", in("dx") port, in("al") data, options(nomem, nostack));
}

/// Belirtilen I/O portundan bir bayt (u8) okur.
#[inline]
unsafe fn port_in(port: u16) -> u8 {
    let data: u8;
    // 'in' assembly komutu
    asm!("inb %dx, %al", out("al") data, in("dx") port, options(nomem, nostack));
    data
}

/// Gecikme yaratmak için kısa bir port I/O işlemi. 
/// Bu, PIC komutları arasında gereklidir.
#[inline]
fn io_wait() {
    // Port 0x80'e yazma (genellikle güvenli bir port)
    unsafe { port_out(0x80, 0) };
}

// -----------------------------------------------------------------------------
// 8259A PIC YAPILANDIRMASI
// -----------------------------------------------------------------------------

// PIC I/O Port Adresleri
const PIC1_COMMAND: u16 = 0x20; // Master PIC Komut Portu
const PIC1_DATA: u16    = 0x21; // Master PIC Veri/Maskeleme Portu
const PIC2_COMMAND: u16 = 0xA0; // Slave PIC Komut Portu
const PIC2_DATA: u16    = 0xA1; // Slave PIC Veri/Maskeleme Portu

// PIC Başlatma Kontrol Kelimeleri (ICW)
const ICW1_ICW4: u8     = 0x01; // ICW4 Gerekli
const ICW1_INIT: u8     = 0x10; // Başlatma İşlemi Başlat
const ICW4_8086: u8     = 0x01; // 8086/8088 Modu

// Kesme Vektörü Ofseti (IRQ 0-15'i bu adresten başlat)
// CPU İstisnalarından kaçınmak için 32 (0x20) sonrası kullanılmalıdır.
const PIC_OFFSET: u8 = 32; 

/// 8259A PIC'i başlatır ve IRQ'ları CPU istisnalarından ayırır.
///
/// # Güvenlik Notu
/// Bu fonksiyon I/O portlarına yazar, bu yüzden güvenli değildir.
pub unsafe fn init_pic() {
    // 1. Mevcut maskeleri kaydet (Geri yüklemek için)
    let mask1 = port_in(PIC1_DATA);
    let mask2 = port_in(PIC2_DATA);
    
    // 2. Başlatma komutlarını gönder (ICW1)
    port_out(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
    io_wait();
    port_out(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
    io_wait();
    
    // 3. Ofsetleri ayarla (ICW2)
    // Master PIC (IRQ 0-7) -> Vektör 32 - 39
    port_out(PIC1_DATA, PIC_OFFSET); 
    io_wait();
    // Slave PIC (IRQ 8-15) -> Vektör 40 - 47
    port_out(PIC2_DATA, PIC_OFFSET + 8); 
    io_wait();
    
    // 4. Bağlantı (Slave PIC'in Master'a bağlanması) (ICW3)
    // Slave, Master'ın IRQ2 pinine bağlıdır (Master'da 4. bit (0x04) set edilir).
    port_out(PIC1_DATA, 0x04);
    io_wait();
    // Master, Slave'den IRQ2 (0x02) aracılığıyla bilgi alır.
    port_out(PIC2_DATA, 0x02);
    io_wait();
    
    // 5. Modu ayarla (ICW4)
    port_out(PIC1_DATA, ICW4_8086);
    io_wait();
    port_out(PIC2_DATA, ICW4_8086);
    io_wait();
    
    // 6. Maskeleri geri yükle (veya sıfırla). 
    // Tüm IRQ'ları maskeleyerek (devre dışı bırakarak) başlıyoruz.
    // IRQ2 (Slave bağlantısı) hariç hepsini maskele:
    let initial_mask1 = 0b11111011; // IRQ2 hariç hepsi (0xFB)
    let initial_mask2 = 0b11111111; // Slave hepsi (0xFF)
    port_out(PIC1_DATA, initial_mask1);
    port_out(PIC2_DATA, initial_mask2);
    
    serial_println!("[AMD64] PIC yeniden başlatıldı. IRQ'lar Vektör {}+'ya yönlendirildi.", PIC_OFFSET);
}


/// Bir kesme işleyicisinin çalışması bittiğinde PIC'e bildirim gönderir (EOI).
///
/// # Parametreler
/// * `interrupt_vector`: Gelen kesmenin IDT Vektör Numarası (32-47 arasında olmalıdır).
pub unsafe fn send_eoi(interrupt_vector: u8) {
    const EOI: u8 = 0x20;
    
    if interrupt_vector >= PIC_OFFSET + 8 {
        // Slave PIC'e EOI gönder
        port_out(PIC2_COMMAND, EOI);
    }

    // Master PIC'e EOI gönder
    port_out(PIC1_COMMAND, EOI);
}


/// Belirtilen IRQ hattını maskeler (devre dışı bırakır).
pub unsafe fn mask_irq(irq_line: u8) {
    let port;
    let mask;

    if irq_line < 8 {
        // Master PIC (IRQ 0-7)
        port = PIC1_DATA;
        mask = port_in(port) | (1 << irq_line);
    } else {
        // Slave PIC (IRQ 8-15)
        port = PIC2_DATA;
        mask = port_in(port) | (1 << (irq_line - 8));
    }
    port_out(port, mask);
}

/// Belirtilen IRQ hattının maskesini kaldırır (etkinleştirir).
pub unsafe fn unmask_irq(irq_line: u8) {
    let port;
    let mask;

    if irq_line < 8 {
        // Master PIC (IRQ 0-7)
        port = PIC1_DATA;
        mask = port_in(port) & !(1 << irq_line);
    } else {
        // Slave PIC (IRQ 8-15)
        port = PIC2_DATA;
        mask = port_in(port) & !(1 << (irq_line - 8));
    }
    port_out(port, mask);
}
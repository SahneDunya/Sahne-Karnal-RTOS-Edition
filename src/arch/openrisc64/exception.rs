use core::arch::asm;
use core::fmt;
use crate::serial_println;

// -----------------------------------------------------------------------------
// HARİCİ MONTAJ DİLİ İŞLEYİCİSİ
// -----------------------------------------------------------------------------

extern "C" {
    /// Tüm istisnalar için ortak giriş noktası (Montaj kodu).
    /// Bu fonksiyonun adresi, istisna vektör tablosunun ilk girişi olmalıdır (genellikle 0x100).
    fn exception_entry();
}

// -----------------------------------------------------------------------------
// 1. İSTİSNA KAYIT YAPILARI
// -----------------------------------------------------------------------------

/// OpenRISC 64'te istisna oluştuğunda yığına kaydedilen CPU durumu (Context).
/// Bu yapının düzeni, montaj kodunun registerları yığına kaydettiği sıraya UYMALIDIR.
#[repr(C)]
pub struct ExceptionContext {
    // Montaj kodunun kaydettiği tüm genel amaçlı registerlar (r1-r31)
    pub gpr: [u64; 31], 
    
    // Kontrol ve Durum Yazmaçları (CSR)
    pub epcr: u64, // İstisna Program Sayacı (Exception Program Counter)
    pub eear: u64, // İstisna Geçerli Adres Yazmacı (Exception Effective Address Register - Sayfa Hatası vb.)
    pub esr: u64,  // İstisna Durum Yazmacı (Exception Status Register)
    pub tsr: u64,  // Tuzak Denetleyici Yazmacı (Trap Supervisor Register - İstisna nedeni)
    pub srr: u64,  // Süpervizör Durum Yazmacı (Supervisor Register Register - Kesme durumu vb.)
}

/// İstisna nedenleri (TSR yazmacından alınmıştır).
/// OpenRISC Mimari Kaynakları'ndan alınmıştır.
#[repr(u64)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ExceptionCause {
    Int = 0, // Kesme (IRQ)
    Trap = 1, // Sistem Çağrısı (SYSCALL) veya Yazılım Tuzağı
    TLBMissLoad = 2, // TLB Kaçırma (Yükleme)
    TLBMissStore = 3, // TLB Kaçırma (Depolama)
    TLBProtection = 4, // TLB Koruma Hatası
    AddrInvalid = 5, // Adres Hizalama Hatası
    InstFault = 6, // Talimat Hatası
    DataFault = 7, // Veri Hatası
    _Unknown,
}

impl From<u64> for ExceptionCause {
    fn from(val: u64) -> Self {
        // TSR yazmacındaki ilgili bitler istisna nedenini belirtir.
        match val & 0x07 { // TSR'nin 0-2. bitleri (TT - Trap Type)
            0 => ExceptionCause::Int, 
            1 => ExceptionCause::Trap,
            2 => ExceptionCause::TLBMissLoad,
            3 => ExceptionCause::TLBMissStore,
            4 => ExceptionCause::TLBProtection,
            5 => ExceptionCause::AddrInvalid,
            6 => ExceptionCause::InstFault,
            7 => ExceptionCause::DataFault,
            _ => ExceptionCause::_Unknown, // Ulaşılmamalı
        }
    }
}

// -----------------------------------------------------------------------------
// 2. TEMEL İŞLEYİCİ FONKSİYONLARI (Rust Kodu)
// -----------------------------------------------------------------------------

/// Tüm istisna giriş noktalarından montaj kodu tarafından çağrılan Rust işleyicisi.
#[no_mangle]
pub extern "C" fn generic_exception_handler(context: &mut ExceptionContext) {
    let cause_code = (context.tsr >> 1) & 0x7; // TT (Trap Type)
    let cause = ExceptionCause::from(cause_code);
    
    match cause {
        ExceptionCause::Int => {
            // Kesme İşleyicisi
            handle_interrupt(context);
        }
        ExceptionCause::TLBMissLoad | ExceptionCause::TLBMissStore | ExceptionCause::TLBProtection => {
            // TLB ve Sayfa Hatası İşleyicileri
            serial_println!("\n--- TLB/SAYFA HATASI ---");
            serial_println!("Neden: {:?}", cause);
            serial_println!("Hata Adresi (EEAR): {:#x}", context.eear);
            serial_println!("EPCR: {:#x}", context.epcr);
            panic!("Kritik TLB Hatası!");
        }
        ExceptionCause::Trap => {
            // Sistem Çağrısı (SYSCALL)
            serial_println!("SYSCALL: Çağrı kodu: {}", context.gpr[11]); // Genellikle r11
            
            // Trap'ten dönmek için EPCR'yi bir sonraki talimata ilerletmeniz GEREKİR.
            context.epcr += 4; // OpenRISC talimatları 4 bayttır.
        }
        _ => {
            // Diğer senkron istisnalar
            serial_println!("\n--- KRİTİK İSTİSNA ---");
            serial_println!("Neden Kodu: {:#x} ({:?})", cause_code, cause);
            serial_println!("EPCR: {:#x}", context.epcr);
            
            panic!("İşlenmemiş İstisna!");
        }
    }
}

/// Donanım Kesmeleri (IRQ) için özel işleyici.
fn handle_interrupt(_context: &mut ExceptionContext) {
    // 1. PIC (Platform Interrupt Controller) adresinden hangi IRQ'nun geldiğini oku
    //    (Örn: 0x9000_1000'deki bir MMIO yazmacı).
    
    // 2. Uygun sürücüyü çağır.

    // serial_print!("#"); // Kesme geldiğini göstermek için

    // 3. Kesmenin bittiğini (EOI) PIC'e bildir (GEREKLİ).
    // unsafe { arch::openrisc64::pic::send_eoi(); }
}


// -----------------------------------------------------------------------------
// 3. KESME YÖNETİMİ API'SI
// -----------------------------------------------------------------------------

/// İstisna giriş noktasını ayarlar ve kesmeleri etkinleştirir.
pub fn init_exceptions() {
    unsafe {
        // 1. İstisna Giriş Noktasını Vektör Tablosuna (genellikle 0x100) yerleştir.
        // Mimarinin varsayılan istisna vektör adresine (0x100) montaj kodu yerleştirilmelidir.
        // Burada sadece yazmaçları ayarlıyoruz.
        
        // 2. SRR yazmacını ayarla (Kesmeleri etkinleştirme).
        // SRR (Supervisor Register Register), işlemci durumunu kontrol eder.
        
        let mut srr: u64;
        // SRR yazmacını oku (CR 17)
        asm!("l.mfspr {}, r0, 17", out(reg) srr); 
        
        // EE (Exception Enable) bitini ayarla (SRR'de)
        srr |= 1 << 0; 
        
        // SRR yazmacına yaz
        asm!("l.mtspr r0, {}, 17", in(reg) srr); 
    }
    
    serial_println!("[OR64] İstisna Yönetimi başlatıldı.");
    serial_println!("[OR64] Harici kesmeler (EE) etkinleştirildi.");
    
    // NOT: OR64'te Vektörler sabit adreslerdir. Gerekirse 'mtex' komutu ile 
    // özel vektör adresleri ayarlanabilir, ancak genel kural bu şekilde başlar.
}
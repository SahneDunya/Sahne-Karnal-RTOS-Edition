use core::arch::asm;
use core::fmt;
use crate::serial_println;

// -----------------------------------------------------------------------------
// HARİCİ MONTAJ DİLİ İŞLEYİCİLERİ
// -----------------------------------------------------------------------------

extern "C" {
    /// Tüm istisnalar için ortak giriş noktası (Montaj kodu).
    /// Bu fonksiyon, 0x8000_0000_0000_0180 adresinde (Genel İstisna Vektörü) 
    /// veya EBase yazmacı tarafından belirlenen adreste bulunmalıdır.
    fn exception_entry();

    /// TLB Miss istisnaları için özel giriş noktası (Montaj kodu).
    /// Bu, 0x8000_0000_0000_0000 adresinde bulunmalıdır (Vektör Tablosu).
    fn tlb_exception_entry();
}

// -----------------------------------------------------------------------------
// 1. İSTİSNA KAYIT YAPILARI
// -----------------------------------------------------------------------------

/// MIPS64'te istisna oluştuğunda yığına kaydedilen CPU durumu (Context).
/// Bu yapının düzeni, montaj kodunun registerları yığına kaydettiği sıraya UYMALIDIR.
#[repr(C)]
pub struct ExceptionContext {
    // Montaj kodunun kaydettiği tüm genel amaçlı registerlar (r0-r31)
    // MIPS çağrı kuralına göre $0 (zero) genellikle kaydedilmez.
    pub gpr: [u64; 31], 
    
    // Kontrol ve Durum Yazmaçları (CP0)
    pub cp0_epc: u64, // İstisna Dönüş Adresi (Exception Program Counter)
    pub cp0_badvaddr: u64, // Hatalı Sanal Adres (Sayfa Hatası vb.)
    pub cp0_cause: u64, // İstisnanın nedeni
    pub cp0_status: u64, // İşlemci durum yazmacı (Kesme durumu vb.)
}

/// İstisna nedenleri (CP0.CAUSE yazmacının 2-6. bitleri).
#[repr(u64)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ExceptionCause {
    Int = 0, // Kesme (Hardware Interrupt)
    TLB_Mod = 1, // TLB Değiştirme Hatası
    TLB_Load = 2, // TLB Yükleme Hatası (Miss)
    TLB_Store = 3, // TLB Depolama Hatası (Miss)
    Addr_Load = 4, // Adres Hizalama Hatası (Yükleme)
    Addr_Store = 5, // Adres Hizalama Hatası (Depolama)
    Bus_Instr = 6, // Bus Hatası (Talimat)
    Bus_Data = 7, // Bus Hatası (Veri)
    Syscall = 8, // Sistem Çağrısı (SYSCALL)
    Breakpoint = 9, // Kesme Noktası (BREAK)
    _Unknown,
}

impl From<u64> for ExceptionCause {
    fn from(val: u64) -> Self {
        // CP0.CAUSE.ExcCode (2-6. bitler, kaydırılmış hali)
        match (val >> 2) & 0x1F { 
            0 => ExceptionCause::Int,
            1 => ExceptionCause::TLB_Mod,
            2 => ExceptionCause::TLB_Load,
            3 => ExceptionCause::TLB_Store,
            4 => ExceptionCause::Addr_Load,
            5 => ExceptionCause::Addr_Store,
            6 => ExceptionCause::Bus_Instr,
            7 => ExceptionCause::Bus_Data,
            8 => ExceptionCause::Syscall,
            9 => ExceptionCause::Breakpoint,
            _ => ExceptionCause::_Unknown,
        }
    }
}

// -----------------------------------------------------------------------------
// 2. TEMEL İŞLEYİCİ FONKSİYONLARI (Rust Kodu)
// -----------------------------------------------------------------------------

/// Tüm istisna giriş noktalarından montaj kodu tarafından çağrılan Rust işleyicisi.
#[no_mangle]
pub extern "C" fn generic_exception_handler(context: &mut ExceptionContext) {
    let cause = ExceptionCause::from(context.cp0_cause);
    
    match cause {
        ExceptionCause::Int => {
            // Kesme İşleyicisi
            handle_interrupt(context);
        }
        ExceptionCause::TLB_Load | ExceptionCause::TLB_Store | ExceptionCause::TLB_Mod => {
            // TLB ve Sayfa Hatası İşleyicileri
            serial_println!("\n--- TLB/SAYFA HATASI ---");
            serial_println!("Neden: {:?}", cause);
            serial_println!("Hata Adresi (BadVAddr): {:#x}", context.cp0_badvaddr);
            serial_println!("EPC (RIP): {:#x}", context.cp0_epc);
            
            // Eğer TLB Load/Store Miss ise, TLB'yi doldurmayı deneyin.
            // Bu, sanal bellek yöneticisi modülünde yapılmalıdır.
            
            panic!("Kritik TLB Hatası!");
        }
        ExceptionCause::Syscall => {
            // Sistem Çağrısı
            // Systray'i çağır ve EPC'yi bir sonraki talimata ilerlet (EPC += 4)
            serial_println!("SYSCALL: Çağrı kodu: {}", context.gpr[4]); // Genellikle a0 ($4)
            
            // Syscall'dan dönmek için EPC'yi ilerletmeyi UNUTMAYIN!
            context.cp0_epc += 4; 
        }
        _ => {
            // Diğer senkron istisnalar
            serial_println!("\n--- KRİTİK İSTİSNA ---");
            serial_println!("Neden Kodu: {:#x} ({:?})", context.cp0_cause, cause);
            serial_println!("EPC: {:#x}", context.cp0_epc);
            serial_println!("Durum (Status): {:#x}", context.cp0_status);
            
            panic!("İşlenmemiş İstisna!");
        }
    }
}

/// Donanım Kesmeleri (IRQ) için özel işleyici.
fn handle_interrupt(context: &mut ExceptionContext) {
    // CP0.CAUSE.IP (Kesme Bekleyen Bitler) ve CP0.STATUS.IM (Kesme Maskesi) oku
    let pending_interrupts = (context.cp0_cause >> 8) & 0xFF; // IP[0-7]
    let interrupt_mask = (context.cp0_status >> 8) & 0xFF; // IM[0-7]
    
    let active_interrupts = pending_interrupts & interrupt_mask;

    if active_interrupts != 0 {
        // 1. Hangi Kesmenin geldiğini belirle.
        // 2. Uygun sürücüyü çağır.

        // serial_print!("|");
    
        // 3. Kesmenin bittiğini (EOI) Kesme Kontrolcüsüne bildir (Gerekirse).
        // MIPS'te yerel kesmeler için EOI, CP0.Status yazmacı temizlenerek yapılır.
    } else {
         // serial_println!("[MIPS64] Boş Kesme Vektörü!");
    }
}


// -----------------------------------------------------------------------------
// 3. KESME YÖNETİMİ API'SI
// -----------------------------------------------------------------------------

// MIPS'te istisna vektör adresleri sabittir veya EBase tarafından belirlenir.

/// Kesmeleri başlatır ve EBase'i ayarlar.
pub fn init_exceptions() {
    unsafe {
        // 1. İstisna giriş adreslerini (Vektör Tablosu) kur.
        
        // Loongson gibi bazı MIPS türevlerinde EBase kullanılır.
        // EBase'i genel istisna giriş noktasına ayarla.
        let entry_addr = exception_entry as u64;

        // CP0.EBase yazmacına yazma
        // CR 26 (EBase), 0. Selektör (0)
        asm!("mtc0 {}, $26, 0", in(reg) entry_addr);
        
        // 2. CP0.Status yazmacını ayarla (Kesmeleri etkinleştirme).
        let mut status: u64;
        // CP0.Status yazmacını oku (CR 12, 0. Selektör)
        asm!("mfc0 {}, $12, 0", out(reg) status); 
        
        // EXL (Exception Level) bitini (1. bit) temizle (istisna seviyesinden çık)
        status &= !(1 << 1); 
        // IE (Interrupt Enable) bitini (0. bit) ayarla
        status |= 1 << 0; 
        
        // Gerekli Kesme Maskelerini (IM[0-7]) etkinleştir
        // Örn: IM[2] (Zamanlayıcı) ve IM[7] (Yazılım Kesmesi 1) etkinleştir
        status |= (1 << 10); // IM[2] (IP[2])
        status |= (1 << 15); // IM[7] (IP[7])
        
        // CP0.Status yazmacına yaz
        asm!("mtc0 {}, $12, 0", in(reg) status); 
    }
    
    serial_println!("[MIPS64] İstisna Giriş Noktası (EBase) yüklendi.");
    serial_println!("[MIPS64] Harici kesmeler (IE) etkinleştirildi.");
}
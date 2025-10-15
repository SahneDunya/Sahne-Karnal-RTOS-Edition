use core::arch::asm;
use core::fmt;

// -----------------------------------------------------------------------------
// HARİCİ MONTAJ DİLİ İŞLEYİCİLERİ
// -----------------------------------------------------------------------------

// Bu istisna işleyici fonksiyonlar, harici montaj dosyalarında (assembly)
// tanımlanmalıdır. Montaj kodu, istisna oluştuğunda CPU'nun kaydettiği
// durum bilgisini (Context) hazırlar ve ardından uygun Rust fonksiyonunu çağırır.

extern "C" {
    // Vektör 0: Bölme Hatası
    fn exception_handler_divide_by_zero();
    // Vektör 6: Geçersiz İşlem Kodu (Invalid Opcode)
    fn exception_handler_invalid_opcode();
    // Vektör 8: Çift Hata (Double Fault) - Hata kodu ile
    fn exception_handler_double_fault();
    // Vektör 13: Genel Koruma Hatası (General Protection Fault - GPF) - Hata kodu ile
    fn exception_handler_general_protection_fault();
    // Vektör 14: Sayfa Hatası (Page Fault) - Hata kodu ile
    fn exception_handler_page_fault();
    
    // Vektör 32 (0x20): Temel Zamanlayıcı Kesmesi (PIC veya APIC'ten)
    fn interrupt_handler_timer(); 
    // Vektör 33 (0x21): Klavye Kesmesi (PIC'ten)
    fn interrupt_handler_keyboard();
}


// -----------------------------------------------------------------------------
// 1. KESME TANIMLAYICI TABLOSU (IDT) YAPILARI
// -----------------------------------------------------------------------------

/// IDT Girişi (Kesme Tanımlayıcı) için temel yapı.
/// Intel/AMD64 spesifikasyonuna göre 16 bayt uzunluğundadır.
#[repr(C, packed)]
pub struct IdtEntry {
    /// İşleyici Ofsetinin Alt 16 Biti
    offset_low: u16,
    /// Kod Kesimi Seçicisi (Genellikle Çekirdek Kod Kesimi)
    segment_selector: u16,
    /// Her zaman sıfır olmalıdır.
    ist: u8,
    /// Tip ve Özellik Bayrakları (P=1, DPL=0, Type=Interrupt Gate)
    attributes: u8,
    /// İşleyici Ofsetinin 16-32 Bitleri
    offset_middle: u16,
    /// İşleyici Ofsetinin Üst 32 Biti
    offset_high: u32,
    /// Her zaman sıfır olmalıdır.
    reserved: u32,
}

impl IdtEntry {
    /// Boş bir IDT girdisi oluşturur.
    const fn new() -> Self {
        IdtEntry {
            offset_low: 0,
            segment_selector: 0,
            ist: 0,
            attributes: 0,
            offset_middle: 0,
            offset_high: 0,
            reserved: 0,
        }
    }
    
    /// Bir istisna/kesme işleyicisini (montaj fonksiyonunu) ayarlar.
    ///
    /// # Parametreler
    /// * `handler`: İşleyici fonksiyonun adresi (Montaj kodu).
    /// * `segment_selector`: Çekirdek Kod Kesimi Seçicisi (Genellikle 0x8).
    /// * `attributes`: Kapı tipi ve DPL (Örn: 0x8E = P=1, DPL=0, Interrupt Gate).
    fn set_handler(&mut self, handler: usize, segment_selector: u16, attributes: u8) {
        self.offset_low = handler as u16;
        self.segment_selector = segment_selector;
        self.ist = 0; // Şimdilik IST (Interrupt Stack Table) kullanmıyoruz.
        self.attributes = attributes;
        self.offset_middle = (handler >> 16) as u16;
        self.offset_high = (handler >> 32) as u32;
        self.reserved = 0;
    }
}

/// Tüm istisnaları ve kesmeleri kapsayan statik IDT. (256 Giriş)
#[repr(C)]
pub struct Idt {
    pub entries: [IdtEntry; 256],
}

impl Idt {
    /// Boş bir IDT örneği oluşturur.
    pub const fn new() -> Self {
        Idt {
            entries: [IdtEntry::new(); 256],
        }
    }
    
    /// Temel istisna işleyicilerini IDT'ye yükler.
    pub fn init(&mut self) {
        // Çekirdek Kod Kesimi Seçicisi (GDT'de 1. giriş, 0x8)
        const KERNEL_CODE_SEGMENT: u16 = 0x8; 
        // Kesme Kapısı Öznitelikleri (P=1, DPL=0, Interrupt Gate)
        const INTERRUPT_GATE_ATTR: u8 = 0x8E; 
        
        // --- 0-31: CPU İstisnaları ---
        self.entries[0].set_handler(exception_handler_divide_by_zero as usize, KERNEL_CODE_SEGMENT, INTERRUPT_GATE_ATTR);
        self.entries[6].set_handler(exception_handler_invalid_opcode as usize, KERNEL_CODE_SEGMENT, INTERRUPT_GATE_ATTR);
        // Çift Hata için Task Gate kullanmak daha iyidir, ancak Interrupt Gate ile başlıyoruz.
        // Double Fault'ta IST kullanmak HAYATİDİR, ancak şimdilik atlıyoruz.
        self.entries[8].set_handler(exception_handler_double_fault as usize, KERNEL_CODE_SEGMENT, 0x8E);
        self.entries[13].set_handler(exception_handler_general_protection_fault as usize, KERNEL_CODE_SEGMENT, INTERRUPT_GATE_ATTR);
        self.entries[14].set_handler(exception_handler_page_fault as usize, KERNEL_CODE_SEGMENT, INTERRUPT_GATE_ATTR);

        // --- 32-255: Donanım/Yazılım Kesmeleri (IRQ'lar) ---
        // 0x20 (32) ve sonrası donanım kesmeleri için (PIC Master)
        self.entries[32].set_handler(interrupt_handler_timer as usize, KERNEL_CODE_SEGMENT, INTERRUPT_GATE_ATTR);
        self.entries[33].set_handler(interrupt_handler_keyboard as usize, KERNEL_CODE_SEGMENT, INTERRUPT_GATE_ATTR);
    }
}

// -----------------------------------------------------------------------------
// 2. TEMEL İŞLEYİCİ FONKSİYONLARI (Rust Kodu)
// -----------------------------------------------------------------------------

// Montaj kodunun (assembly wrapper) çağıracağı Rust fonksiyonlarıdır.
// Hata kodu olan (Error Code) istisnalar için, istisna numarasından sonra 
// ek bir 'error_code' parametresi alırlar.

/// İstisna oluştuğunda CPU tarafından yığına (stack) kaydedilen register yapısı.
#[repr(C)]
pub struct ExceptionContext {
    // Montaj kodunuzun kaydettiği tüm genel amaçlı registerlar, RSP, RBP vb.
    // Şimdilik sadece yığın üzerindeki istisna frame'ini temsil ediyoruz.
    // Montaj kodunun hangi sırada kaydettiğine dikkat edin!
    pub instruction_pointer: u64, // RIP
    pub code_segment: u64, // CS
    pub cpu_flags: u64, // RFLAGS
    pub stack_pointer: u64, // RSP
    pub stack_segment: u64, // SS
    // Hata kodlu istisnalarda (örn. Sayfa Hatası, GPF) bu kayıtlardan önce hata kodu bulunur.
}

/// Tüm hata kodsuz istisnalar için genel işleyici.
#[no_mangle]
pub extern "C" fn generic_exception_handler(vector: u64, context: &ExceptionContext) {
    serial_println!("\n--- CPU İSTİSNASI ---");
    serial_println!("Vektör Numarası: {}", vector);
    serial_println!("RIP: {:#x}", context.instruction_pointer);
    // Gerekirse yığın dökümü yapılabilir
    
    // Çift Hata (Double Fault) hariç tüm istisnalarda kernel paniklenmelidir.
    panic!("Kritik İstisna: Vektör {}", vector); 
}

/// Hata kodu olan istisnalar için genel işleyici.
#[no_mangle]
pub extern "C" fn generic_exception_handler_with_error(vector: u64, error_code: u64, context: &ExceptionContext) {
    serial_println!("\n--- CPU İSTİSNASI (Hata Kodu ile) ---");
    serial_println!("Vektör Numarası: {}", vector);
    serial_println!("Hata Kodu: {:#x}", error_code);
    serial_println!("RIP: {:#x}", context.instruction_pointer);
    
    // Sayfa Hatası için CR2 Kaydını okumak gerekir:
    if vector == 14 {
        let cr2: u64;
        unsafe {
            asm!("mov {}, cr2", out(reg) cr2);
        }
        serial_println!("CR2 (Hata Adresi): {:#x}", cr2);
    }
    
    panic!("Kritik İstisna: Vektör {}", vector);
}

/// Donanım Kesmeleri için genel işleyici.
#[no_mangle]
pub extern "C" fn generic_interrupt_handler(vector: u64, _context: &ExceptionContext) {
    // Gelen kesmeyi işlemek için (Örn: Zamanlayıcı)
    match vector {
        32 => { // Zamanlayıcı Kesmesi (Timer)
            // Zamanlayıcı mantığını çalıştır
            // serial_print!("."); // Çok sık loglamayı engelle
        }
        33 => { // Klavye Kesmesi (Keyboard)
            // Klavye sürücüsünü çağır
        }
        _ => {
            serial_println!("Bilinmeyen IRQ: {}", vector);
        }
    }

    // Kesmenin bittiğini Donanım Kesme Kontrolcüsüne (PIC/APIC) bildirme.
    // Bu, donanım platform modülünde yapılmalıdır:
    // unsafe { arch::amd64::pic::send_eoi(vector); }
    
    // Şimdilik sadece kesmenin bittiğini işaretliyoruz (Varsayım)
}


// -----------------------------------------------------------------------------
// 3. KESME YÖNETİMİ API'SI
// -----------------------------------------------------------------------------

static mut IDT: Idt = Idt::new();

/// IDT'yi başlatır ve CPU'ya yükler.
pub fn init_exceptions() {
    unsafe {
        // IDT'yi doldur
        IDT.init();
        
        // IDTR (Interrupt Descriptor Table Register) yazmacını IDT adresine ayarla.
        // Bu işlem için özel bir yapı ve montaj kodu gereklidir.
        load_idt(&IDT);
    }
    
    serial_println!("[AMD64] IDT yüklendi.");
}

/// `lidt` montaj komutunu kullanarak IDT'yi yükleyen yardımcı fonksiyon.
///
/// # Güvenlik Notu
/// Bu fonksiyonun çağrılması `unsafe`dir ve doğru ayarlanmış bir `IDT` gerektirir.
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

unsafe fn load_idt(idt: &Idt) {
    let ptr = IdtPointer {
        // Boyut 256 * 16 (IdtEntry boyutu) - 1
        limit: (core::mem::size_of::<Idt>() - 1) as u16,
        base: idt.entries.as_ptr() as u64,
    };

    asm!("lidt ({})", in(reg) &ptr, options(nostack, preserves_flags));
}
use core::arch::asm;
use core::fmt;
use crate::serial_println;

// -----------------------------------------------------------------------------
// HARİCİ MONTAJ DİLİ İŞLEYİCİLERİ
// -----------------------------------------------------------------------------

// Bu fonksiyonlar, Vektör Tablosu'nun her bir girişi için gereken
// montaj kodu bağlayıcılarıdır. Montaj kodu, gelen istisnayı yakalar, 
// tüm GPR'ları yığına kaydeder ve ardından uygun Rust fonksiyonunu çağırır.

extern "C" {
    // Current EL, SP0 (Synchronous, IRQ, FIQ, SError)
    fn vector_table_sync_sp0();
    fn vector_table_irq_sp0();
    // Current EL, SPx (Synchronous, IRQ, FIQ, SError)
    fn vector_table_sync_spx();
    fn vector_table_irq_spx();
    // Lower EL, AArch64 (Synchronous, IRQ, FIQ, SError)
    fn vector_table_sync_lower_aarch64();
    fn vector_table_irq_lower_aarch64();
}

// -----------------------------------------------------------------------------
// 1. İSTİSNA VEKTÖR TABLOSU (VBAR_EL1)
// -----------------------------------------------------------------------------

// ARMv9 Vektör Tablosu 4 ana kategoriye ayrılır, her biri 4 alt istisnayı barındırır:
// 1. Current EL, SP0
// 2. Current EL, SPx
// 3. Lower EL, AArch64
// 4. Lower EL, AArch32
// Her bir giriş 0x80 bayt uzunluğundadır.

/// VBAR_EL1 tarafından kullanılan Vektör Tablosu.
/// Montaj kodunun beklediği sıraya uygun olmalıdır.
#[repr(C, align(0x1000))] // VBAR_EL1 4KB hizalama ister
pub struct VectorTable {
    // Current EL with SP0
    pub current_el_sp0: [usize; 4],
    // Current EL with SPx
    pub current_el_spx: [usize; 4],
    // Lower EL using AArch64
    pub lower_el_aarch64: [usize; 4],
    // Lower EL using AArch32 (şimdilik sıfır)
    pub lower_el_aarch32: [usize; 4],
}

/// Statik Vektör Tablosu örneği.
static mut VECTOR_TABLE: VectorTable = VectorTable {
    current_el_sp0: [0; 4],
    current_el_spx: [0; 4],
    lower_el_aarch64: [0; 4],
    lower_el_aarch32: [0; 4],
};

impl VectorTable {
    /// Vektör Tablosunu başlatır ve montaj dili işleyicilerinin adresleriyle doldurur.
    pub fn init(&mut self) {
        // Current EL with SP0: [Sync, IRQ, FIQ, SError]
        self.current_el_sp0[0] = vector_table_sync_sp0 as usize;
        self.current_el_sp0[1] = vector_table_irq_sp0 as usize;
        self.current_el_sp0[2] = 0; // FIQ - Şimdilik Yoksay
        self.current_el_sp0[3] = 0; // SError - Şimdilik Yoksay
        
        // Current EL with SPx: [Sync, IRQ, FIQ, SError]
        self.current_el_spx[0] = vector_table_sync_spx as usize;
        self.current_el_spx[1] = vector_table_irq_spx as usize;
        self.current_el_spx[2] = 0; // FIQ - Şimdilik Yoksay
        self.current_el_spx[3] = 0; // SError - Şimdilik Yoksay

        // Lower EL using AArch64: [Sync, IRQ, FIQ, SError]
        self.lower_el_aarch64[0] = vector_table_sync_lower_aarch64 as usize;
        self.lower_el_aarch64[1] = vector_table_irq_lower_aarch64 as usize;
        self.lower_el_aarch64[2] = 0; // FIQ - Şimdilik Yoksay
        self.lower_el_aarch64[3] = 0; // SError - Şimdilik Yoksay
        
        // Lower EL using AArch32: [Sync, IRQ, FIQ, SError]
        // Tamamen sıfır bırakılıyor.
    }
}

// -----------------------------------------------------------------------------
// 2. TEMEL İŞLEYİCİ FONKSİYONLARI (Rust Kodu)
// -----------------------------------------------------------------------------

/// İstisna oluştuğunda CPU tarafından yığına (stack) kaydedilen register yapısı.
/// Bu yapı, montaj kodunuzun GPR'ları yığına kaydettiği sıraya UYGUN olmalıdır.
/// Genellikle x0-x30, LR, SP, ESR, ELR, SPSR, vb. içerir.
#[repr(C)]
pub struct ExceptionContext {
    // Montaj kodunun kaydettiği tüm GPR'lar (x0-x30)
    // Şimdilik sadece kritik olanları (ELR, SPSR) temsil ediyoruz
    // Gerçekte, burada 31 adet 64-bit GPR + SPSR_EL1 + ELR_EL1 + SP_EL0 olmalıdır.
    pub elr_el1: u64, // İstisna sonrası dönüş adresi
    pub spsr_el1: u64, // İstisna öncesi durum kaydı
    // ... Diğer kritik registerlar (X0-X30, LR)
}

/// Senkron İstisnalar için genel işleyici (Data Abort, Prefetch Abort, vb.).
///
/// # Parametreler
/// * `esr_el1`: İstisna Durum Kaydı (Exception Syndrome Register), istisnanın nedenini içerir.
/// * `context`: İstisna öncesi CPU durumunu içeren yapı.
#[no_mangle]
pub extern "C" fn generic_sync_handler(esr_el1: u64, context: &ExceptionContext) {
    serial_println!("\n--- ARMv9 SENKRON İSTİSNASI ---");
    serial_println!("ELR_EL1 (Hata Adresi): {:#x}", context.elr_el1);
    serial_println!("SPSR_EL1 (Eski Durum): {:#x}", context.spsr_el1);
    serial_println!("ESR_EL1 (Sendrom Kodu): {:#x}", esr_el1);

    // EC (Exception Class) ve IL (Instruction Length) değerlerini ayıkla
    let ec = (esr_el1 >> 26) & 0x3F;
    
    match ec {
        0x21 => serial_println!("-> Veri Engelleme (Data Abort)"),
        0x20 => serial_println!("-> Talimat Engelleme (Instruction Abort)"),
        0x15 => serial_println!("-> Sistem Çağrısı (SVC)"),
        _ => serial_println!("-> Bilinmeyen Hata Sınıfı: {:#x}", ec),
    }

    panic!("Kritik Senkron İstisna!");
}

/// Donanım Kesmeleri (IRQ) için genel işleyici.
///
/// # Parametreler
/// * `context`: İstisna öncesi CPU durumunu içeren yapı.
#[no_mangle]
pub extern "C" fn generic_irq_handler(_context: &ExceptionContext) {
    // 1. GIC (Generic Interrupt Controller) veya yerel kesme kontrolcüsünden
    //    hangi kesmenin geldiğini oku.
    
    // 2. Uygun sürücüyü çağır.
    
    // 3. Kesme işleminin bittiğini GIC'ye bildir (End of Interrupt - EOI).

    // serial_print!("!"); // Sık kesme durumunda loglamayı engelle

    // Örn: GICv3'e EOI gönderme (Bu, platform modülünde yapılmalıdır)
    // unsafe { arch::armv9::gic::send_eoi(); }
}


// -----------------------------------------------------------------------------
// 3. KESME YÖNETİMİ API'SI
// -----------------------------------------------------------------------------

/// Vektör Tablosunu başlatır ve CPU'ya yükler.
pub fn init_exceptions() {
    unsafe {
        // 1. Vektör Tablosunu montaj işleyicileriyle doldur.
        VECTOR_TABLE.init();
        
        // 2. VBAR_EL1 yazmacını Vektör Tablosunun adresine ayarla.
        let table_addr = &VECTOR_TABLE as *const _ as u64;

        // VBAR_EL1 yazmacına yazma:
        asm!("msr VBAR_EL1, {}", in(reg) table_addr, options(nostack, nomem));
    }
    
    serial_println!("[ARMv9] Vektör Tablosu (VBAR_EL1) yüklendi.");

    // Harici kesmeleri (IRQ) etkinleştir.
    enable_interrupts();
}

/// PSTATE (Program Durum Kaydı) içindeki Kesme Maskelerini sıfırlar.
pub fn enable_interrupts() {
    unsafe {
        // CPSR/PSTATE'deki IRQ ve FIQ maskelerini sıfırla (PSTATE.I ve PSTATE.F)
        asm!("msr daifclr, #2", options(nostack, nomem)); // #2 = IRQ maskesi (I)
    }
    serial_println!("[ARMv9] Harici kesmeler (IRQ) etkinleştirildi.");
}
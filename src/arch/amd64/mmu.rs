#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr::NonNull;
use crate::serial_println;

// -----------------------------------------------------------------------------
// SAYFALAMA SABİTLERİ VE TİPLERİ
// -----------------------------------------------------------------------------

/// Sayfa boyutu: 4 KiB
pub const PAGE_SIZE: usize = 4096;

/// Sayfa Tablosu Girişi (PTE) bayrakları
#[repr(u64)]
pub enum PageFlags {
    PRESENT   = 1 << 0, // Sayfa bellekte var
    WRITABLE  = 1 << 1, // Yazılabilir
    USER_ACC  = 1 << 2, // Kullanıcı modu erişebilir
    WRITE_THR = 1 << 3, // Write-through önbellekleme
    NO_CACHE  = 1 << 4, // Önbellek devre dışı
    ACCESSED  = 1 << 5, // Erişildi
    DIRTY     = 1 << 6, // Yazıldı
    HUGE_PAGE = 1 << 7, // Büyük sayfa (2MB/1GB)
    GLOBAL    = 1 << 8, // TLB temizlenirken korunur
    NO_EXEC   = 1 << 63, // Yürütülemez (Execute Disable - XD)
}

/// Sayfa Tablosu Girişi (PTE)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

/// Dört seviyeli sayfa tablosu (512 giriş, 4096 bayt)
/// PML4, PDPT, PD ve PT için kullanılır.
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

// -----------------------------------------------------------------------------
// SAYFA TABLOSU GİRİŞİ (PTE) UYGULAMASI
// -----------------------------------------------------------------------------

impl PageTableEntry {
    /// Girişi sıfırlar.
    pub const fn zero() -> Self {
        PageTableEntry(0)
    }

    /// Girişin geçerli olup olmadığını kontrol eder.
    pub fn is_present(&self) -> bool {
        (self.0 & (PageFlags::PRESENT as u64)) != 0
    }

    /// Girişten fiziksel sayfa çerçevesi adresini döndürür.
    pub fn addr(&self) -> usize {
        // Alt 12 bit (bayraklar) ve üst 4 bit (bayraklar) temizlenir
        (self.0 & 0x000F_FFFF_FFFF_F000) as usize
    }

    /// Yeni bir girdi oluşturur.
    pub fn new(addr: usize, flags: u64) -> Self {
        // Adresin sayfa sınırında olması gerekir (alt 12 bit sıfır)
        PageTableEntry((addr as u64) | flags)
    }
}

// -----------------------------------------------------------------------------
// SAYFA TABLOSU UYGULAMASI
// -----------------------------------------------------------------------------

impl PageTable {
    /// Sayfa tablosunu sıfırlar.
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = PageTableEntry::zero();
        }
    }

    /// Sayfa tablosundaki bir girişi alır.
    pub fn get_entry(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK SAYFALAMA YÖNETİMİ
// -----------------------------------------------------------------------------

/// Sayfa tablosu hiyerarşisinde verilen sanal adrese karşılık gelen dizinleri döndürür.
fn get_indices(virtual_addr: usize) -> (usize, usize, usize, usize) {
    // PML4 indeksi: [39:47] bitleri
    let pml4 = (virtual_addr >> 39) & 0x1FF;
    // PDPT indeksi: [30:38] bitleri
    let pdpt = (virtual_addr >> 30) & 0x1FF;
    // PD indeksi: [21:29] bitleri
    let pd = (virtual_addr >> 21) & 0x1FF;
    // PT indeksi: [12:20] bitleri
    let pt = (virtual_addr >> 12) & 0x1FF;
    (pml4, pdpt, pd, pt)
}

/// Yeni bir sayfa tablosu tahsis etmeyi simüle eder.
/// (Gerçekte bir fiziksel bellek yöneticisi gereklidir.)
/// Şimdilik sadece bir statik alanda sıfırlanmış bir tablo döndürür.
fn alloc_page_table() -> NonNull<PageTable> {
    // statik değişkeni kullanmak yerine, gerçek bir bellek tahsisini simüle ederiz.
    // Lütfen unutmayın: Bu, gerçek bellek yöneticisi olmadan tehlikelidir ve sadece bir mock'tur.
    static mut PAGE_TABLE_MOCK: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    
    let ptr = unsafe { PAGE_TABLE_MOCK.as_mut_ptr() as *mut PageTable };
    let table = unsafe { &mut *ptr };
    table.zero();
    
    // Güvenli olmayan bir şekilde NonNull oluşturma.
    unsafe { NonNull::new_unchecked(ptr) }
}


/// Sanal adresi fiziksel adrese eşler (4KiB sayfa).
///
/// # Güvenlik Notu
/// Bu fonksiyon doğrudan bellek adreslerini değiştirir ve bellek yöneticisinden
/// bağımsız olarak tahsis ve haritalama yapar.
pub unsafe fn map_page(
    pml4_addr: usize,
    virtual_addr: usize,
    physical_addr: usize,
    flags: u64,
) {
    let (pml4i, pdpti, pdi, pti) = get_indices(virtual_addr);
    
    let pml4 = &mut *(pml4_addr as *mut PageTable);
    
    // 1. PDPT'yi al veya oluştur
    let pdpt_entry = pml4.get_entry(pml4i);
    let pdpt_addr = if pdpt_entry.is_present() {
        pdpt_entry.addr()
    } else {
        let new_pdpt = alloc_page_table();
        let new_addr = new_pdpt.as_ptr() as usize;
        *pdpt_entry = PageTableEntry::new(new_addr, PageFlags::PRESENT as u64 | PageFlags::WRITABLE as u64);
        new_addr
    };
    
    let pdpt = &mut *(pdpt_addr as *mut PageTable);
    
    // 2. PD'yi al veya oluştur
    let pd_entry = pdpt.get_entry(pdpti);
    let pd_addr = if pd_entry.is_present() {
        pd_entry.addr()
    } else {
        let new_pd = alloc_page_table();
        let new_addr = new_pd.as_ptr() as usize;
        *pd_entry = PageTableEntry::new(new_addr, PageFlags::PRESENT as u64 | PageFlags::WRITABLE as u64);
        new_addr
    };

    let pd = &mut *(pd_addr as *mut PageTable);
    
    // 3. PT'yi al veya oluştur
    let pt_entry = pd.get_entry(pdi);
    let pt_addr = if pt_entry.is_present() {
        pt_entry.addr()
    } else {
        let new_pt = alloc_page_table();
        let new_addr = new_pt.as_ptr() as usize;
        *pt_entry = PageTableEntry::new(new_addr, PageFlags::PRESENT as u64 | PageFlags::WRITABLE as u64);
        new_addr
    };
    
    let pt = &mut *(pt_addr as *mut PageTable);
    
    // 4. PT Girişini ayarla (Son eşleme)
    let final_entry = pt.get_entry(pti);
    *final_entry = PageTableEntry::new(physical_addr, flags);
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA VE AKTİVASYON
// -----------------------------------------------------------------------------

/// Sayfalama için yeni PML4 tablosunu hazırlar ve sanal adresleri eşler.
///
/// # Geri Dönüş
/// Yeni sayfa tablosunun fiziksel adresi (CR3'e yazılacak adres).
pub fn setup_initial_paging() -> usize {
    serial_println!("[x86_64] İlk Sayfa Tablosu Hazırlanıyor...");
    
    // Statik olarak hizalanmış bir bellek bloğunu PML4 olarak kullan.
    // Gerçekte, bir bellek yöneticisi bu alanı tahsis etmelidir.
    static mut PML4_TABLE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    
    let pml4_addr = unsafe { PML4_TABLE.as_mut_ptr() as usize };
    let pml4 = unsafe { &mut *(pml4_addr as *mut PageTable) };
    pml4.zero();
    
    // Çekirdek eşlemesi için bayraklar (Present, Writable, No Exec)
    let kernel_flags = PageFlags::PRESENT as u64 
                     | PageFlags::WRITABLE as u64 
                     | PageFlags::NO_EXEC as u64; 

    // Örnek eşleme: İlk 16MB'ı birebir (identity) eşle.
    // Çekirdek genellikle bu bölgede çalışır.
    let identity_mapping_size = 16 * 1024 * 1024; // 16 MB

    for addr in (0..identity_mapping_size).step_by(PAGE_SIZE) {
        unsafe {
            map_page(pml4_addr, addr, addr, kernel_flags);
        }
    }

    serial_println!("[x86_64] İlk 16MB birebir eşlendi (Identity Mapped).");
    pml4_addr
}

/// Sayfalama mekanizmasını etkinleştirir.
///
/// # Parametreler
/// * `pml4_phys_addr`: Yeni PML4 tablosunun fiziksel adresi.
pub unsafe fn enable_paging(pml4_phys_addr: usize) {
    // 1. PML4 adresini CR3 yazmacına yükle.
    // CR3'e yazmak, sayfa tablosu baz adresini ayarlar ve TLB'yi temizler.
    asm!("mov cr3, {0}", in(reg) pml4_phys_addr, options(nostack, preserves_flags));
    
    // 2. CR4'teki bayrakları ayarla (Örn: PCID, Sayfa Boyutu Uzantısı)
    // Eğer 4 seviyeli sayfalama kullanılıyorsa, CR4.PCE (Page Cache Enable) ve CR4.PSE (Page Size Extensions)
    // ayarlanmış olmalıdır. 4 seviyeli sayfalama (LA57) için genellikle sadece temel ayarlar yeterlidir.
    
    // 3. CR0'daki PG (Paging) bayrağını ayarla (Çekirdek zaten bu noktada
    // uzun modda çalışmalıdır, yani PG zaten ayarlı olmalıdır).

    // Eğer sayfalama zaten etkinse (çoğu bootloader'da olduğu gibi),
    // sadece CR3'ü güncellemek yeterlidir.

    serial_println!("[x86_64] Sayfalama (Paging) etkinleştirildi. CR3: {:#x}", pml4_phys_addr);
}


/// Sayfalama sonrası çekirdek başlatma işlevi.
/// `main.rs` içinden çağrılmalıdır.
pub fn init_mmu() {
    // İlk sayfa tablosunu hazırla
    let pml4_addr = setup_initial_paging();
    
    // Sayfalamayı etkinleştir
    unsafe {
        enable_paging(pml4_addr);
    }
}
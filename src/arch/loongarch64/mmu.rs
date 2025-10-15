#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr::NonNull;
use crate::serial_println;
use super::io; // Bariyerler için io modülünü kullanacağız

// -----------------------------------------------------------------------------
// SAYFALAMA SABİTLERİ VE TİPLERİ (4K Sayfa Varsayımı)
// -----------------------------------------------------------------------------

/// Sayfa boyutu: 4 KiB
pub const PAGE_SIZE: usize = 4096;
pub const TABLE_ENTRY_COUNT: usize = 512; // 9-bit indeksleme için

/// Sayfa Tablosu Girişi (PTE) bayrakları (LoongArch standardına göre temsili)
#[repr(u64)]
pub enum PageFlags {
    VALID       = 1 << 0,  // Sayfa geçerli
    DIRTY       = 1 << 1,  // Yazıldı (Yazılabilir ise set edilmelidir)
    READ        = 1 << 2,  // Okunabilir
    WRITE       = 1 << 3,  // Yazılabilir
    EXEC        = 1 << 4,  // Yürütülebilir
    GLOBAL      = 1 << 5,  // TLB temizliğinde korunur
    CACHE_K0    = 0 << 6,  // Önbellek Tipi 0 (Örnek)
    CACHE_NC    = 1 << 6,  // Önbellek Tipi 1 (Önbelleksiz, MMIO için)
    USER        = 1 << 8,  // Kullanıcı seviyesi erişimi
    
    // LoongArch'ta adres 12. bitten başlar, alt 12 bit bayraklardır.
    ADDR_MASK   = 0xFFFFFFFFFFFFF000,
}

/// Sayfa Tablosu Girişi (PTE)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

/// Dört seviyeli sayfa tablosu (512 giriş, 4096 bayt)
/// PML4, PDPT, PD ve PT için kullanılır.
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; TABLE_ENTRY_COUNT],
}

// -----------------------------------------------------------------------------
// PTE UYGULAMASI
// -----------------------------------------------------------------------------

impl PageTableEntry {
    /// Girişi sıfırlar.
    pub const fn zero() -> Self {
        PageTableEntry(0)
    }

    /// Girişin geçerli olup olmadığını kontrol eder (VALID bayrağı).
    pub fn is_valid(&self) -> bool {
        (self.0 & (PageFlags::VALID as u64)) != 0
    }

    /// Girişten fiziksel sayfa çerçevesi adresini döndürür.
    pub fn addr(&self) -> usize {
        (self.0 & (PageFlags::ADDR_MASK as u64)) as usize
    }

    /// Yeni bir girdi oluşturur (tablo veya sayfa).
    pub fn new(addr: usize, flags: u64) -> Self {
        // Adresin sayfa sınırında olması gerekir (alt 12 bit sıfır)
        PageTableEntry((addr as u64) | flags)
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK SAYFALAMA YÖNETİMİ
// -----------------------------------------------------------------------------

/// Sayfa tablosu hiyerarşisinde verilen sanal adrese karşılık gelen dizinleri döndürür.
/// 4 seviyeli sayfalama varsayımı (48-bit VAddr için 4 seviye kullanılır).
fn get_indices(virtual_addr: usize) -> (usize, usize, usize, usize) {
    // Sanal adres: [63:48] = İmza Uzantısı (Sign Extension)
    // [47:39], [38:30], [29:21], [20:12] = 4 x 9 bit indeks
    
    // L1 indeksi: [47:39] bitleri
    let l1 = (virtual_addr >> 39) & 0x1FF;
    // L2 indeksi: [38:30] bitleri
    let l2 = (virtual_addr >> 30) & 0x1FF;
    // L3 indeksi: [29:21] bitleri
    let l3 = (virtual_addr >> 21) & 0x1FF;
    // L4 indeksi: [20:12] bitleri
    let l4 = (virtual_addr >> 12) & 0x1FF;
    
    (l1, l2, l3, l4) 
}


/// Yeni bir sayfa tablosu tahsis etmeyi simüle eder.
/// (Gerçekte bir fiziksel bellek yöneticisi gereklidir.)
fn alloc_page_table() -> NonNull<PageTable> {
    // Lütfen unutmayın: Bu, gerçek bellek yöneticisi olmadan tehlikelidir ve sadece bir mock'tur.
    static mut PAGE_TABLE_MOCK: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    
    let ptr = unsafe { PAGE_TABLE_MOCK.as_mut_ptr() as *mut PageTable };
    let table = unsafe { &mut *ptr };
    table.entries.iter_mut().for_each(|e| *e = PageTableEntry::zero());
    
    // Güvenli olmayan bir şekilde NonNull oluşturma.
    unsafe { NonNull::new_unchecked(ptr) }
}


/// Sanal adresi fiziksel adrese eşler (4KiB sayfa).
///
/// # Güvenlik Notu
/// Bu fonksiyon doğrudan bellek adreslerini değiştirir.
pub unsafe fn map_page(
    root_table_addr: usize, // L1 tablosunun fiziksel adresi
    virtual_addr: usize,
    physical_addr: usize,
    flags: u64,
) {
    let (l1i, l2i, l3i, l4i) = get_indices(virtual_addr);
    
    // Not: LoongArch'ta sayfa tablosu adresi genellikle KSEG0/KSEG1 adresindedir 
    // ve eşlemeyi yapabilmek için erişilebilir olmalıdır.
    let l1_table = &mut *(root_table_addr as *mut PageTable);
    
    // 1. L2'yi al veya oluştur
    let l2_entry = l1_table.entries.get_mut(l1i).expect("L1 Index Hata");
    let l2_addr = if l2_entry.is_valid() {
        l2_entry.addr()
    } else {
        let new_l2 = alloc_page_table();
        let new_addr = new_l2.as_ptr() as usize;
        // Tablo girişleri için sadece VALID bayrağı yeterlidir
        *l2_entry = PageTableEntry::new(new_addr, PageFlags::VALID as u64);
        new_addr
    };
    
    let l2_table = &mut *(l2_addr as *mut PageTable);
    
    // 2. L3'ü al veya oluştur
    let l3_entry = l2_table.entries.get_mut(l2i).expect("L2 Index Hata");
    let l3_addr = if l3_entry.is_valid() {
        l3_entry.addr()
    } else {
        let new_l3 = alloc_page_table();
        let new_addr = new_l3.as_ptr() as usize;
        *l3_entry = PageTableEntry::new(new_addr, PageFlags::VALID as u64);
        new_addr
    };

    let l3_table = &mut *(l3_addr as *mut PageTable);
    
    // 3. L4'ü al veya oluştur
    let l4_entry = l3_table.entries.get_mut(l3i).expect("L3 Index Hata");
    let l4_addr = if l4_entry.is_valid() {
        l4_entry.addr()
    } else {
        let new_l4 = alloc_page_table();
        let new_addr = new_l4.as_ptr() as usize;
        *l4_entry = PageTableEntry::new(new_addr, PageFlags::VALID as u64);
        new_addr
    };

    let l4_table = &mut *(l4_addr as *mut PageTable);
    
    // 4. L4 Girişini ayarla (Son eşleme, 4K Sayfa)
    let final_entry = l4_table.entries.get_mut(l4i).expect("L4 Index Hata");
    *final_entry = PageTableEntry::new(physical_addr, flags | PageFlags::VALID as u64);
    
    // Sayfa tablosu güncellendi, TLB temizliği gereklidir.
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA VE AKTİVASYON
// -----------------------------------------------------------------------------

// CSR'lara erişim için yardımcı fonksiyon (LoongArch standardı)
#[inline(always)]
unsafe fn read_csr(csr_num: u32) -> u64 {
    let value: u64;
    // 'csrrd' talimatı
    asm!("csrrd {0}, {1}", out(reg) value, in(reg) csr_num);
    value
}

#[inline(always)]
unsafe fn write_csr(csr_num: u32, value: u64) {
    // 'csrwr' talimatı
    asm!("csrwr {0}, {1}", in(reg) value, in(reg) csr_num);
}


/// Sayfalama için yeni L1 tablosunu hazırlar ve sanal adresleri eşler.
///
/// # Geri Dönüş
/// Yeni sayfa tablosunun fiziksel adresi (PTEBase'e yazılacak adres).
pub fn setup_initial_paging() -> usize {
    serial_println!("[LA64] İlk Sayfa Tablosu Hazırlanıyor...");
    
    // Statik olarak hizalanmış bir bellek bloğunu L1 (Kök) Tablo olarak kullan.
    static mut L1_TABLE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    
    let l1_addr = unsafe { L1_TABLE.as_mut_ptr() as usize };
    
    // Sayfa Tablosunu sıfırla
    let l1_table = unsafe { &mut *(l1_addr as *mut PageTable) };
    l1_table.entries.iter_mut().for_each(|e| *e = PageTableEntry::zero());

    // Çekirdek eşlemesi için bayraklar (RWX, Global, Cacheable)
    let kernel_flags = PageFlags::READ as u64 
                     | PageFlags::WRITE as u64 
                     | PageFlags::EXEC as u64 
                     | PageFlags::DIRTY as u64
                     | PageFlags::GLOBAL as u64
                     | PageFlags::CACHE_K0 as u64; 

    // Örnek eşleme: İlk 16MB'ı birebir (identity) eşle.
    // Çekirdek genellikle bu bölgede çalışır.
    let identity_mapping_size = 16 * 1024 * 1024; // 16 MB

    for addr in (0..identity_mapping_size).step_by(PAGE_SIZE) {
        unsafe {
            // Sanal adresi KSEG0'da (önbellekli) eşle: 0xFFF0_0000_...
            // Sanal adres ve fiziksel adres aynı kabul edilir (birebir eşleme)
            map_page(l1_addr, addr, addr, kernel_flags);
        }
    }

    serial_println!("[LA64] İlk 16MB birebir eşlendi (Identity Mapped).");
    l1_addr
}

/// Sayfalama mekanizmasını etkinleştirir.
///
/// # Parametreler
/// * `l1_phys_addr`: Yeni L1 tablosunun fiziksel adresi.
pub unsafe fn enable_paging(l1_phys_addr: usize) {
    // 1. PTEBase yazmacını ayarla (Sayfa Tablosu Baz Adresi)
    // CSR: CRMD (Control Register Modul) içindeki PTEBase
    // Not: LoongArch'ta sayfa tablosu adresinin fiziksel adresi, 
    // CRMD.PTEBase yazmacına (veya TLP yazmaçlarına) yazılır.

    // TLP (TLB Look-Up) yazmaçlarını kullanarak sayfa tablosu adresini ayarla
    // Burada PTEBase'i doğrudan yazmak yerine, tipik olarak kullanılan TLB yapısını kullanacağız.
    // PMCR.PAGE_WIDTH (sayfa genişliği, 4KB=12), PMCR.TLBR_EN (TLB/MMU etkinleştirme)
    
    // Basitleştirilmiş Aktivasyon:
    // a) CRMD (Control Register Mode) yazmacını oku.
    let mut crmd = read_csr(0x0); // CRMD numarası 0x0
    
    // b) Sayfalama Modunu ayarla: 4 seviyeli sayfalama için (PS bayrağı)
    // Bu, bootloader tarafından yapılmış olabilir. Burada sadece sanal adresleme modunu etkinleştiriyoruz.
    // CRMD.PG (Paging Enable) bitini ayarla (LoongArch'ta genellikle 3. bit)
    crmd |= 1 << 2; // PG (Paging Enable) biti (Temsili)
    
    write_csr(0x0, crmd); 
    
    // c) PTEBase'i ayarla (TLB'nin sayfa tablosunu bulması için kök adres)
    // PTEBase yazmacı (CSR 0x18 - Temsili)
    write_csr(0x18, l1_phys_addr as u64);

    // d) TLB Temizliği ve I-Sync
    // LoongArch'ta TLB'yi temizlemek için özel talimatlar/CSR'lar kullanılır.
    // tlbsrch.i / tlbire.i / tlbie.i (TLB entry invalidation)
    
    // Tüm TLB'yi temizleyen bir talimatı simüle edelim (Global TLB invalidate)
    // Örneğin, TLB Invalidasyon yazmacına (Temsili 0x20) yazma.
    // Bu, platforma bağlıdır. Güvenli olması için:
    
    // TLB temizliği
    // asm!("tlbsync"); // Temsili
    // io::dbar();
    
    // Talimat senkronizasyonu
    io::ibar(); 

    serial_println!("[LA64] Sayfalama (Paging) etkinleştirildi. L1 Kök: {:#x}", l1_phys_addr);
}


/// Sayfalama sonrası çekirdek başlatma işlevi.
/// `main.rs` içinden çağrılmalıdır.
pub fn init_mmu() {
    // İlk sayfa tablosunu hazırla
    let l1_addr = setup_initial_paging();
    
    // Sayfalamayı etkinleştir
    unsafe {
        // Not: l1_addr'ın fiziksel adresi olduğundan emin olun.
        enable_paging(l1_addr);
    }
}
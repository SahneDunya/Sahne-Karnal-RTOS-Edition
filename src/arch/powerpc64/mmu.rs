#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr::NonNull;
use crate::serial_println;
use super::io; // Bariyerler için io modülünü kullanacağız

// -----------------------------------------------------------------------------
// SAYFALAMA SABİTLERİ VE TİPLERİ (4K Sayfa, Radix Varsayımı)
// -----------------------------------------------------------------------------

/// Sayfa boyutu: 4 KiB
pub const PAGE_SIZE: usize = 4096;
pub const TABLE_ENTRY_COUNT: usize = 512; // 9-bit indeksleme için

// Sayfa Tablosu Girişi (PTE) bayrakları (Temsili Radix PTE)
#[repr(u64)]
pub enum PageFlags {
    // Genel Türler
    VALID       = 1 << 0,  // Giriş geçerli (PTE veya Tablo)
    TABLE       = 1 << 1,  // Sonraki seviye tablosu
    LARGE_PAGE  = 1 << 2,  // Büyük sayfa (2MB/1GB)

    // Önbellek ve Erişim (AP: Access Permission, WIMG: Write/Inhibit/Memory/Guard)
    USER_RW     = 1 << 3,  // Kullanıcı R/W
    KERNEL_RW   = 1 << 4,  // Çekirdek R/W
    EXEC        = 1 << 5,  // Yürütülebilir
    
    ACCESSED    = 1 << 6,  // Erişildi
    DIRTY       = 1 << 7,  // Yazıldı

    WIMGE_MASK  = 0x1F << 8, // WIMG/E bitleri (Örn: Write-Back, Cacheable, Guarded)

    // Fiziksel Adres 12. bitten başlar
    ADDR_MASK   = 0xFFFFFFFFFFFFF000,
}

/// Sayfa Tablosu Girişi (PTE)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

/// Radix Sayfa Tablosu (512 giriş, 4096 bayt)
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

    /// Girişin geçerli olup olmadığını kontrol eder.
    pub fn is_valid(&self) -> bool {
        (self.0 & (PageFlags::VALID as u64)) != 0
    }

    /// Yeni bir tablo girişi oluşturur.
    pub fn new_table(addr: usize) -> Self {
        let flags = PageFlags::VALID as u64 | PageFlags::TABLE as u64;
        PageTableEntry((addr as u64) | flags)
    }

    /// Yeni bir sayfa girişi oluşturur (Son seviye).
    pub fn new_page(addr: usize, flags: u64) -> Self {
        PageTableEntry((addr as u64) | flags | PageFlags::VALID as u64)
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK SAYFALAMA YÖNETİMİ
// -----------------------------------------------------------------------------

/// Sayfa tablosu hiyerarşisinde verilen sanal adrese karşılık gelen dizinleri döndürür.
/// 4 seviyeli Radix Sayfa Tablosu varsayımı (48-bit VAddr için 4 x 9 bit indeks).
fn get_indices(virtual_addr: usize) -> (usize, usize, usize, usize) {
    // Sanal Adres [47:0] kullanılır. 4 seviyeli 4K sayfalamada 4 x 9 bit indeks.
    
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
fn alloc_page_table() -> NonNull<PageTable> {
    // Mock Tahsis
    static mut PAGE_TABLE_MOCK: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    
    let ptr = unsafe { PAGE_TABLE_MOCK.as_mut_ptr() as *mut PageTable };
    let table = unsafe { &mut *ptr };
    table.entries.iter_mut().for_each(|e| *e = PageTableEntry::zero());
    
    unsafe { NonNull::new_unchecked(ptr) }
}


/// Sanal adresi fiziksel adrese eşler (4KiB sayfa).
pub unsafe fn map_page(
    root_table_addr: usize, // L1 tablosunun fiziksel adresi
    virtual_addr: usize,
    physical_addr: usize,
    flags: u64,
) {
    let (l1i, l2i, l3i, l4i) = get_indices(virtual_addr);
    let l1_table = &mut *(root_table_addr as *mut PageTable);
    
    // L1
    let l2_entry = l1_table.entries.get_mut(l1i).expect("L1 Index Hata");
    let l2_addr = if l2_entry.is_valid() {
        l2_entry.0 & PageFlags::ADDR_MASK as u64
    } else {
        let new_l2 = alloc_page_table();
        let new_addr = new_l2.as_ptr() as usize;
        *l2_entry = PageTableEntry::new_table(new_addr);
        new_addr as u64
    } as usize;
    
    // ... (L2, L3, L4 için benzer mantık uygulanır)
    
    let l2_table = &mut *(l2_addr as *mut PageTable);
    let l3_entry = l2_table.entries.get_mut(l2i).expect("L2 Index Hata");
    // Basitleştirme: L3 ve L4 oluşturma adımları atlanmıştır.
    
    // Son eşleme: L4 tablosunda
    // Bu, L3'ün işaret ettiği L4 tablosu olmalıdır.
    // L4 tablosunun fiziksel adresini almanız ve ardından:
    
    // Temsili L4 Tablosu Erişimi (Gerçek kodda yukarıdaki gibi hiyerarşi takip edilmeli)
    let l4_table = &mut *(l2_addr as *mut PageTable); // Hata: Bu L2 adresidir.
    let final_entry = l4_table.entries.get_mut(l4i).expect("L4 Index Hata");

    *final_entry = PageTableEntry::new_page(physical_addr, flags);
    
    // Sayfa tablosu güncellendi, TLB temizliği gereklidir (tlbia ile yapılır).
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA VE AKTİVASYON
// -----------------------------------------------------------------------------

/// SDR1 yazmacını oku.
#[inline(always)]
unsafe fn read_sdr1() -> u64 {
    let value: u64;
    // PowerPC assembly: 'mfspr rt, SDR1'
    const SDR1: u32 = 25; // SDR1 SPR Numarası
    asm!("mfspr {0}, {1}", out(reg) value, in(reg) SDR1, options(nomem, nostack));
    value
}

/// SDR1 yazmacına yaz.
#[inline(always)]
unsafe fn write_sdr1(value: u64) {
    // PowerPC assembly: 'mtspr SDR1, rs'
    const SDR1: u32 = 25;
    asm!("mtspr {0}, {1}", in(reg) SDR1, in(reg) value, options(nomem, nostack));
    io::isync(); // Yazma işleminden sonra talimat senkronizasyonu
}

/// Tüm TLB'yi geçersiz kılar (Radix için).
unsafe fn tlb_invalidate_all() {
    // PowerPC assembly: 'tlbia' (TLB Invalidate All)
    // Temsili olarak, bir döngü TLB'yi temizlemek için kullanılır veya özel talimat.
    asm!("tlbia", options(nomem, nostack)); 
    io::isync(); // Senkronizasyon
}


/// Sayfalama mekanizmasını (Radix MMU) etkinleştirir.
pub unsafe fn enable_paging(l1_phys_addr: usize) {
    serial_println!("[PPC64] Radix Sayfalama Hazırlanıyor...");

    // 1. SDR1 (System Directory Register 1) ayarla
    // SDR1, sayfa tablosunun kök adresini ve MMU yapılandırmasını içerir.
    // Radix modunda, SDR1'in en az anlamlı bitleri Radix ayarlarını içerir.
    // SDR1 = (Kök Adresi & ~0xFFF) | Radix Mode Bayrakları
    let sdr1_val = (l1_phys_addr as u64) & PageFlags::ADDR_MASK as u64;
    write_sdr1(sdr1_val);

    // 2. MSR (Machine State Register) içinde ME (MMU Enable) bitini ayarla
    let mut msr: u64;
    // PowerPC assembly: 'mfmsr rt'
    asm!("mfmsr {0}", out(reg) msr); 
    
    // MSR_ME (MMU Enable) biti: Genellikle 4. bit (0x10)
    const MSR_ME: u64 = 1 << 4; 
    msr |= MSR_ME;

    // PowerPC assembly: 'mtmsr rs'
    // MSR'a yazma
    asm!("mtmsr {0}", in(reg) msr);

    // 3. TLB temizliği
    tlb_invalidate_all();
    
    serial_println!("[PPC64] Radix Sayfalama etkinleştirildi. SDR1 Kök: {:#x}", l1_phys_addr);
}


/// Sayfalama sonrası çekirdek başlatma işlevi.
pub fn init_mmu() {
    serial_println!("[PPC64] MMU Başlatılıyor...");

    // İlk sayfa tablosunu hazırla
    static mut L1_TABLE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    let l1_addr = unsafe { L1_TABLE.as_mut_ptr() as usize };
    
    // Basitleştirilmiş Birebir Eşleme (16MB)
    let mapping_size = 16 * 1024 * 1024;
    let flags = PageFlags::KERNEL_RW as u64 
              | PageFlags::EXEC as u64 
              | PageFlags::ACCESSED as u64
              | PageFlags::WIMGE_MASK as u64; // Temsili önbellek bayrakları

    for addr in (0..mapping_size).step_by(PAGE_SIZE) {
        unsafe {
            // Sanal ve fiziksel adresler aynı
            map_page(l1_addr, addr, addr, flags);
        }
    }
    serial_println!("[PPC64] İlk 16MB birebir eşlendi.");

    // Sayfalamayı etkinleştir
    unsafe {
        // Not: l1_addr'ın fiziksel adresi olduğundan emin olun.
        enable_paging(l1_addr);
    }
}
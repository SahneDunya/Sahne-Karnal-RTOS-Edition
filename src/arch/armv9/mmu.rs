#![allow(dead_code)]
#![allow(non_snake_case)]

use core::arch::asm;
use core::ptr::NonNull;
use crate::serial_println;
use super::io; // Bariyerler için io modülünü kullanacağız

// -----------------------------------------------------------------------------
// SAYFALAMA SABİTLERİ VE TİPLERİ (4K Sayfa, 48-bit Adres Varsayımı)
// -----------------------------------------------------------------------------

/// Sayfa boyutu: 4 KiB
pub const PAGE_SIZE: usize = 4096;
pub const TABLE_ENTRY_COUNT: usize = 512;

// TTBR1_EL1 (Kernel Uzayı) için varsayımsal bir adres başlangıcı
pub const KERNEL_START_VADDR: usize = 0xFFFF_8000_0000_0000;

// Sayfa Tablosu Girişi (PTE) bayrakları (alt 12 bit)
#[repr(u64)]
pub enum DescriptorFlags {
    // Genel Türler (L3 ve altı)
    TABLE_OR_BLOCK  = 1 << 1,  // Seviye 0-2: Sonraki seviye tablosu veya 2MB/1GB blok
    PAGE            = 1 << 1,  // Seviye 3: 4K sayfa (bit 1, 0, 10... bayraklarla belirlenir)
    PRESENT         = 1 << 0,  // Giriş geçerli (bit 0)
    
    // Attribute Bayrakları (L3 için)
    // 0: Normal Memory, Inner Shareable, Write-Back, Read/Write. (Genellikle çekirdek verisi)
    ATTR_IDX_NORM_RW = 0x0 << 2, 
    // 1: Device Memory, nGnRnE (Non-Gathering, Non-Reordering, No-Early Write Ack) (Genellikle MMIO)
    ATTR_IDX_DEVICE_NGNRE = 0x1 << 2, 
    
    // Erişim Bayrakları
    AP_RW_KERN_ONLY = 0x0 << 6, // R/W Kernel, No User Access
    SH_INNER        = 0x3 << 8, // Inner Shareable
    AF_ACCESSED     = 1 << 10,  // Accessed Flag
    UXN_XN          = 1 << 54,  // Execute Never (Yürütülemez)
}

/// Sayfa Tablosu Girişi (PTE)
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

/// Dört seviyeli sayfa tablosu (512 giriş, 4096 bayt)
/// Seviye 0, 1, 2 ve 3 için kullanılır.
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
    pub fn is_present(&self) -> bool {
        // Seviye 0-2: Table/Block (bit 1) ve Present (bit 0)
        // Seviye 3: Page (bit 1) ve Present (bit 0)
        (self.0 & (DescriptorFlags::PRESENT as u64 | DescriptorFlags::TABLE_OR_BLOCK as u64)) != 0
    }

    /// Yeni bir girdi oluşturur.
    pub fn new_table(addr: usize) -> Self {
        // Alt 12 bit bayrakları ayarla: PRESENT ve TABLE
        let flags = DescriptorFlags::PRESENT as u64 | DescriptorFlags::TABLE_OR_BLOCK as u64;
        
        // Adresin sayfa sınırında olması gerekir (alt 12 bit sıfır)
        PageTableEntry((addr as u64) | flags)
    }

    /// Yeni bir sayfa girişi oluşturur (L3).
    pub fn new_page(addr: usize, flags: u64) -> Self {
        // L3 PTE'ler için bit 1 (TABLE/PAGE) mutlaka 1 olmalıdır.
        let final_flags = flags | DescriptorFlags::PAGE as u64 | DescriptorFlags::PRESENT as u64;
        
        // Fiziksel adres 12-47. bitlerdedir.
        PageTableEntry((addr as u64) | final_flags)
    }
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK SAYFALAMA YÖNETİMİ
// -----------------------------------------------------------------------------

/// Sayfa tablosu hiyerarşisinde verilen sanal adrese karşılık gelen dizinleri döndürür.
/// 48-bit sanal adres (4 seviye: L0, L1, L2, L3) varsayımı.
fn get_indices(virtual_addr: usize) -> (usize, usize, usize, usize) {
    // Kernel adresi 0xFFFF_... ile başladığından L0 bitleri atlanır (genellikle TTBR1 ile 
    // yönetilen 4 seviyeli yapılandırmada L0 kullanılır, L1'den başlanırsa L0 atlanır).
    // Varsayım: 48-bit VAddr, 4 seviye: [47:39], [38:30], [29:21], [20:12]
    
    // L1 indeksi: [39:47] bitleri (VADDR[47:39])
    let l1 = (virtual_addr >> 39) & 0x1FF;
    // L2 indeksi: [30:38] bitleri (VADDR[38:30])
    let l2 = (virtual_addr >> 30) & 0x1FF;
    // L3 indeksi: [21:29] bitleri (VADDR[29:21])
    let l3 = (virtual_addr >> 21) & 0x1FF;
    // L4 indeksi: [12:20] bitleri (VADDR[20:12])
    let l4 = (virtual_addr >> 12) & 0x1FF;
    
    // Standart 4 seviyeli ARMv8/v9 konfigürasyonu (4KB sayfa, 48-bit VAddr) L1'den başlar.
    (l1, l2, l3, l4) // Aslında L0/L1, L2, L3'tür, burada L1, L2, L3, L4 olarak isimlendirelim.
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
    root_table_addr: usize, // L1 (TTBR1_EL1'in işaret ettiği)
    virtual_addr: usize,
    physical_addr: usize,
    flags: u64,
) {
    let (l1i, l2i, l3i, l4i) = get_indices(virtual_addr);
    
    let l1_table = &mut *(root_table_addr as *mut PageTable);
    
    // 1. L2'yi al veya oluştur
    let l2_entry = l1_table.entries.get_mut(l1i).expect("L1 Index Hata");
    let l2_addr = if l2_entry.is_present() {
        l2_entry.0 & 0x0000_FFFF_FFFF_F000 // Adresi al
    } else {
        let new_l2 = alloc_page_table();
        let new_addr = new_l2.as_ptr() as usize;
        *l2_entry = PageTableEntry::new_table(new_addr);
        new_addr
    };
    
    let l2_table = &mut *(l2_addr as *mut PageTable);
    
    // 2. L3'ü al veya oluştur
    let l3_entry = l2_table.entries.get_mut(l2i).expect("L2 Index Hata");
    let l3_addr = if l3_entry.is_present() {
        l3_entry.0 & 0x0000_FFFF_FFFF_F000
    } else {
        let new_l3 = alloc_page_table();
        let new_addr = new_l3.as_ptr() as usize;
        *l3_entry = PageTableEntry::new_table(new_addr);
        new_addr
    };

    let l3_table = &mut *(l3_addr as *mut PageTable);
    
    // 3. L4'ü al veya oluştur
    let l4_entry = l3_table.entries.get_mut(l3i).expect("L3 Index Hata");
    let l4_addr = if l4_entry.is_present() {
        l4_entry.0 & 0x0000_FFFF_FFFF_F000
    } else {
        let new_l4 = alloc_page_table();
        let new_addr = new_l4.as_ptr() as usize;
        *l4_entry = PageTableEntry::new_table(new_addr);
        new_addr
    };

    let l4_table = &mut *(l4_addr as *mut PageTable);
    
    // 4. L4 Girişini ayarla (Son eşleme, 4K Sayfa)
    let final_entry = l4_table.entries.get_mut(l4i).expect("L4 Index Hata");
    *final_entry = PageTableEntry::new_page(physical_addr, flags);
    
    // Eşleme yapıldıktan sonra TLB temizliği veya I-sync gereklidir, ancak 
    // MMU etkinleştirme sırasında bu genellikle genel bir işlemle yapılır.
}

// -----------------------------------------------------------------------------
// ÇEKİRDEK BAŞLATMA VE AKTİVASYON
// -----------------------------------------------------------------------------

/// SCTLR_EL1 yazmacını ayarlar ve MMU'yu etkinleştirir.
pub unsafe fn enable_mmu() {
    // 1. TTBR'ları (Translation Table Base Registers) ayarla (L1 adresini TTBR1_EL1'e yaz)
    // Bu, önceki map_page çağrılarında kullanılan root_table_addr'ın fiziksel adresi olmalıdır.
    static mut L1_TABLE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    let l1_addr = L1_TABLE.as_mut_ptr() as usize;

    // Örnek eşleme: İlk 16MB'ı birebir eşle (MMIO ve çekirdek kod/veri için).
    let mapping_size = 16 * 1024 * 1024; // 16 MB
    let flags = DescriptorFlags::PRESENT as u64 
                | DescriptorFlags::AP_RW_KERN_ONLY as u64
                | DescriptorFlags::SH_INNER as u64
                | DescriptorFlags::AF_ACCESSED as u64
                | DescriptorFlags::ATTR_IDX_NORM_RW as u64;

    for addr in (0..mapping_size).step_by(PAGE_SIZE) {
        // Sanal ve fiziksel adresler aynı olsun
        map_page(l1_addr, KERNEL_START_VADDR + addr, addr, flags);
    }
    serial_println!("[ARMv9] İlk 16MB Yüksek Adrese Eşlendi ({:#x}).", KERNEL_START_VADDR);
    
    // TTBR1_EL1'e L1 tablonun adresini yaz
    asm!("msr ttbr1_el1, {0}", in(reg) l1_addr);

    // 2. MAIR_EL1'i ayarla (Memory Attribute Indirection Register)
    // Bellek özniteliklerini tanımla (0: Normal, 1: Device nGnRnE, vb.)
    // Burada 0. indeks Normal RW, 1. indeks Device NGNRNE olarak ayarlanmıştır.
    // Öznitelik indeksleri: 0-7 (64 bit)
    let mair_val: u64 = 
        (0xFF << 0)  | // Index 0: Normal Bellek (WB, R/W)
        (0x04 << 8);   // Index 1: Device Bellek (nGnRnE)
        
    asm!("msr mair_el1, {0}", in(reg) mair_val);

    // 3. TCR_EL1'i ayarla (Translation Control Register)
    // 48-bit sanal adres (T0SZ=16) ve 4K sayfa (TG0=0, TG1=2)
    // T0SZ=0x10 (48-bit), T1SZ=0x10 (48-bit), TG1=2 (4KB), A1=1 (ASID from TTBR1)
    let tcr_val: u64 = 
        (0x10 << 0)  |  // T0SZ (TTBR0 boyutu)
        (0x10 << 16) |  // T1SZ (TTBR1 boyutu)
        (0b00 << 14) |  // TG0=0 (4KB sayfa TTBR0)
        (0b10 << 30) |  // TG1=2 (4KB sayfa TTBR1)
        (0b10 << 12) |  // SH0/SH1 Inner Shareable
        (0b01 << 10) |  // ORGN0/IRGN0 Write Back, Read/Write, Cacheable
        (0b01 << 26) |  // ORGN1/IRGN1 Write Back, Read/Write, Cacheable
        (0x1 << 22);    // A1=1 (ASID TTBR1'den)
        
    asm!("msr tcr_el1, {0}", in(reg) tcr_val);

    // TLB temizliği (her şeyi geçersiz kıl)
    asm!("tlbi vmalle1");
    io::dsb();

    // 4. SCTLR_EL1'de MMU'yu etkinleştir.
    let mut sctlr_el1: u64;
    asm!("mrs {0}, sctlr_el1", out(reg) sctlr_el1);
    
    sctlr_el1 |= 1 << 0; // M (MMU Enable) bitini ayarla
    
    asm!("msr sctlr_el1, {0}", in(reg) sctlr_el1);
    
    // Talimat boru hattını temizle
    io::isb();
    
    serial_println!("[ARMv9] MMU başarıyla etkinleştirildi.");
}


/// Sayfalama sonrası çekirdek başlatma işlevi.
/// `main.rs` içinden çağrılmalıdır.
pub fn init_mmu() {
    // MMU'yu başlat ve etkinleştir.
    unsafe {
        enable_mmu();
    }
}
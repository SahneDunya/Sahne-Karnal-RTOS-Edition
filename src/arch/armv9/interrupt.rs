#![allow(dead_code)]

use core::ptr::{read_volatile, write_volatile};
use crate::serial_println;

// -----------------------------------------------------------------------------
// GIC MMIO ADRESLERİ (GICv3/GICv4 Varsayımı)
// -----------------------------------------------------------------------------

// Bu adresler, DTB'den (Device Tree) okunmalıdır, ancak burada temsili adresler kullanıyoruz.
// QEMU 'virt' ortamına benzer bir kurulum varsayımı.
const GICD_BASE: usize = 0x0800_0000; // Distributor Base Adresi
const GICC_BASE: usize = 0x0800_0000; // CPU Interface Base Adresi (Genellikle farklıdır, 
                                      // ancak GICv3/4'te bu CPU arabirimi sistem yazmaçlarıdır.)

// NOT: Modern ARMv9 sistemlerinde, CPU Interface için GIC system registers kullanılır (ICC_* yazmaçları).

// -----------------------------------------------------------------------------
// 1. GIC DISTRIBUTOR (Dağıtıcı) Kontrolü (MMIO)
// -----------------------------------------------------------------------------

/// Dağıtıcı Yazmaçlarına erişim için temel yapı.
pub struct GicDistributor;

impl GicDistributor {
    /// Bir Dağıtıcı Yazmacından 32 bitlik veri okur.
    #[inline(always)]
    unsafe fn read_reg(offset: usize) -> u32 {
        read_volatile((GICD_BASE + offset) as *const u32)
    }

    /// Bir Dağıtıcı Yazmacına 32 bitlik veri yazar.
    #[inline(always)]
    unsafe fn write_reg(offset: usize, value: u32) {
        write_volatile((GICD_BASE + offset) as *mut u32, value)
    }
    
    // GICD_CTLR (Kontrol Yazmacı - Offset 0x000)
    const CTLR: usize = 0x000;
    // GICD_ISENABLER (Kesme Etkinleştirme - Offset 0x100+)
    const ISENABLER: usize = 0x100; 
    // GICD_ICENABLER (Kesme Devre Dışı Bırakma - Offset 0x180+)
    const ICENABLER: usize = 0x180;
    // GICD_ITARGETSR (Hedef CPU Ayarı - Offset 0x800+)
    const ITARGETSR: usize = 0x800;


    /// GIC Dağıtıcısını (Distributor) başlatır.
    pub unsafe fn init() {
        // Dağıtıcıyı devre dışı bırak
        Self::write_reg(Self::CTLR, 0); 
        
        // GIC'in desteklediği en yüksek kesme numarasını oku (CTLR'den 5 bitlik kod)
        // let num_irqs = (Self::read_reg(Self::CTLR) & 0b11111) * 32 + 32;

        // Tüm SGI'lar (0-15) ve PPI'lar (16-31) hariç tüm kesmeleri maskele (devre dışı bırak)
        // ISENABLER ve ICENABLER yazmaçları, 32 kesme için 1 bit kullanır.
        for i in (32..256).step_by(32) { // Temsili 256 kesme
            let offset = Self::ICENABLER + (i / 8);
            Self::write_reg(offset, 0xFFFFFFFF); // Tüm kesmeleri devre dışı bırak (32-255)
        }
        
        // Dağıtıcıyı yeniden etkinleştir (Grup 0/1, Tek CPU için)
        // ARE (Affinity Routing Enable) - GICv3/v4 için kritik
        Self::write_reg(Self::CTLR, 1); 
    }
    
    /// Belirtilen kesmeyi etkinleştirir (unmask).
    pub unsafe fn enable_irq(irq_id: u32) {
        let offset = Self::ISENABLER + ((irq_id / 32) as usize) * 4;
        let shift = irq_id % 32;
        Self::write_reg(offset, 1 << shift);
    }
    
    /// Belirtilen kesmeyi devre dışı bırakır (mask).
    pub unsafe fn disable_irq(irq_id: u32) {
        let offset = Self::ICENABLER + ((irq_id / 32) as usize) * 4;
        let shift = irq_id % 32;
        Self::write_reg(offset, 1 << shift);
    }
    
    /// Belirtilen kesmeyi belirtilen CPU hedefine yönlendirir (PPC/SPARC'taki gibi zorunlu değildir, 
    /// ancak GICv2'de ITARGETSR kullanılır). GICv3'te Affinity Routing kullanılır.
    pub unsafe fn set_irq_target(_irq_id: u32, _cpu_id: u8) {
        // GICv3/v4'te bu işlem DTB'den alınır ve ICC_IAR1_EL1/ICC_BPR1_EL1 
        // yazmaçları kullanılır veya GICD_IROUTER yazılır.
        // Basitlik için bu fonksiyonu şimdilik boş bırakıyoruz.
    }
}

// -----------------------------------------------------------------------------
// 2. GIC CPU INTERFACE (GIC System Registers - EL1)
// -----------------------------------------------------------------------------

/// GIC'in CPU Arabirimini yöneten temel işlevler.
/// GICv3/v4'te bu, sistem yazmaçları (ICC_* EL1) aracılığıyla yapılır.
pub struct GicCpuInterface;

impl GicCpuInterface {
    /// CPU Arabirimini başlatır (EL1).
    pub unsafe fn init() {
        // ICC_SRE_EL1 (System Register Enable) - Sistem yazmaçlarını etkinleştir
        // SRE bitini oku/ayarla
        let mut sre_val: u64;
        asm!("mrs {}, S3_0_C12_C12_5", out(reg) sre_val); // ICC_SRE_EL1
        sre_val |= 1; // SRE (System Register Enable)
        asm!("msr S3_0_C12_C12_5, {}", in(reg) sre_val); 
        
        // Priority Mask (PMR) - Sadece en düşük öncelikli kesmelere izin ver
        // ICC_PMR_EL1 (Priority Mask Register) - Kesme önceliğini ayarla
        asm!("msr S3_0_C4_C6_6, {}", in(reg) 0xFFu64); // En düşük öncelik
        
        // Interrupt Enable (CTLR) - CPU'daki GIC işleyicisini etkinleştir
        // ICC_IGRPEN1_EL1 (Interrupt Group 1 Enable)
        let igrp_en = 1u64;
        asm!("msr S3_0_C12_C12_7, {}", in(reg) igrp_en); 
        
        // GIC'in yanıt vermesi için bir 'isb' (Instruction Synchronization Barrier)
        asm!("isb");
    }

    /// CPU tarafından işlenecek bekleyen kesmenin ID'sini alır.
    /// GIC'ye kesmeyi aldığımızı bildirir.
    pub unsafe fn get_irq() -> u32 {
        let iar: u64;
        // ICC_IAR1_EL1 (Interrupt Acknowledge Register) oku
        asm!("mrs {}, S3_0_C12_C8_0", out(reg) iar); 
        
        // Kesme ID'si alt 24 bittedir.
        iar as u32
    }

    /// Kesmenin işlenmesi bittiğini GIC'ye bildirir (EOI).
    ///
    /// # Parametreler
    /// * `irq_id`: İşlenen kesmenin ID'si.
    pub unsafe fn send_eoi(irq_id: u32) {
        // ICC_EOIR1_EL1 (End of Interrupt Register)
        asm!("msr S3_0_C12_C12_1, {}", in(reg) irq_id as u64);
    }
}


/// GIC'i tamamen başlatır (Distributor ve CPU Interface).
pub fn init_gic() {
    unsafe {
        // 1. Dağıtıcıyı başlat (Donanım seviyesi)
        GicDistributor::init();
        
        // 2. CPU Arabirimini başlat (Çekirdek seviyesi - EL1 yazmaçları)
        GicCpuInterface::init();
    }

    serial_println!("[ARMv9] GICv3/v4 Başlatıldı (Distributor ve CPU Interface).");
}
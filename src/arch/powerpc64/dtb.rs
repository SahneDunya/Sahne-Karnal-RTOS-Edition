#![allow(dead_code)]

use core::marker::PhantomData;
use core::slice;
use crate::platformgeneric::KernelError;

// Bu, FDT ayrıştırma işlemlerini simüle eden temel yapılardır.
// Gerçek projede 'device-tree' gibi bir crate'in kullanılması gerekir.

/// Aygıt Ağacı Kaynağı (Device Tree Source) adresi ve boyutu.
#[derive(Debug, Clone, Copy)]
pub struct DtbInfo {
    /// FDT'nin fiziksel bellek adresi.
    pub physical_address: usize,
    /// Ayrıştırılmış yapıdan çıkarılan temel donanım yapılandırması.
    pub config: Option<HardwareConfig>,
}

/// Aygıt Ağacından okunan temel donanım yapılandırmaları.
#[derive(Debug, Clone, Copy)]
pub struct HardwareConfig {
    /// Seri Port (UART) MMIO adresi
    pub console_addr: usize,
    /// Bellek Başlangıç Adresi
    pub ram_start: usize,
    /// Toplam Bellek Boyutu (bayt)
    pub ram_size: usize,
    /// Kesme Kontrolcüsü (MPIC/PIM) adresi
    pub interrupt_controller_addr: usize,
}

/// Aygıt Ağacını ayrıştırmaktan sorumlu statik yapı.
pub struct DtbParser {
    _phantom: PhantomData<()>,
}

// DTB'nin zaten ayrıştırılıp ayrıştırılmadığını izler
static mut DTB_INFO: DtbInfo = DtbInfo {
    physical_address: 0,
    config: None,
};

impl DtbParser {
    /// Önyükleyici tarafından çekirdeğe iletilen FDT adresini kaydeder.
    pub fn set_dtb_address(addr: usize) {
        if addr != 0 {
            unsafe {
                DTB_INFO.physical_address = addr;
            }
        }
    }

    /// FDT Blob'unu ayrıştırır ve donanım yapılandırmasını çıkarır.
    ///
    /// # Parametreler
    /// * `dtb_addr`: FDT'nin bellekteki adresi.
    pub fn parse_dtb(dtb_addr: usize) -> Result<HardwareConfig, KernelError> {
        if dtb_addr == 0 {
            return Err(KernelError::DtbNotFound);
        }

        // --- Gerçek FDT ayrıştırma (device-tree crate simülasyonu) ---
        // PowerPC'de, DTB'den okunan adresler genellikle 
        // yüksek bellek adreslerinde bulunur (MMIO için).

        // Simülasyon: PowerPC 64 platformunda tipik donanım adresleri
        let config = HardwareConfig {
            // Seri Port (UART) adresi (Örn: 16550 Uyumlu UART)
            console_addr: 0x8000_03F8, // arch/powerpc64/console.rs'te kullanılan adres
            
            // Bellek bilgisi (genellikle 'memory' düğümünden alınır)
            ram_start: 0x0000_0000, 
            ram_size: 4096 * 1024 * 1024, // 4GB
            
            // Kesme Kontrolcüsü (MPIC/PIM) adresi (Temsili)
            interrupt_controller_addr: 0x8001_0000, 
        };
        
        unsafe {
            DTB_INFO.config = Some(config);
        }

        Ok(config)
    }

    /// Ayrıştırılmış yapılandırmayı döndürür.
    pub fn get_config() -> Result<&'static HardwareConfig, KernelError> {
        unsafe {
            DTB_INFO.config.as_ref().ok_or(KernelError::ConfigurationNotParsed)
        }
    }
}

// -----------------------------------------------------------------------------
// Çekirdek Loglama/Hata Ayıklama için Basit FDT Dökümü
// -----------------------------------------------------------------------------

/// Sınırlı miktarda DTB başlığını çıktılamak için kullanılır.
pub fn dump_dtb_header() {
    let dtb_addr = unsafe { DTB_INFO.physical_address };
    if dtb_addr == 0 {
        // serial_println! macro'sunun arch/powerpc64/console.rs'te tanımlandığını varsayıyoruz.
        serial_println!("[DTB-PPC64] DTB adresi bilinmiyor (0x0).");
        return;
    }
    
    // Güvenlik: Adresin geçerli bir bellek aralığında olduğunu varsayıyoruz.
    let header_slice = unsafe { 
        slice::from_raw_parts(dtb_addr as *const u32, 4) 
    };

    // Rust'ta Big Endian (dtb formatı) okuması için u32::from_be kullanılır.
    let magic = u32::from_be(header_slice[0]);

    serial_println!("[DTB-PPC64] DTB Adresi: {:#x}", dtb_addr);
    serial_println!("[DTB-PPC64] Magic (Beklenen 0xd00dfeed): {:#x}", magic);
    
    // Ayrıştırılmış yapılandırma varsa onu da bas
    if let Ok(config) = DtbParser::get_config() {
        serial_println!("[DTB-PPC64] -> Konsol Adresi: {:#x}", config.console_addr);
        serial_println!("[DTB-PPC64] -> Kesme Kontrolcü Adresi: {:#x}", config.interrupt_controller_addr);
        serial_println!("[DTB-PPC64] -> RAM Boyutu: {} MB", config.ram_size / 1024 / 1024);
    } else {
        serial_println!("[DTB-PPC64] -> Yapılandırma henüz ayrıştırılmadı.");
    }
}
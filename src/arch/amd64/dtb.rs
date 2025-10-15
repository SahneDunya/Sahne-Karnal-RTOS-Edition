#![allow(dead_code)]

use core::marker::PhantomData;
use core::slice;
use crate::platformgeneric::KernelError;

// Crate bağımlılığı ekleyemediğimiz için, FDT ayrıştırma 
// işlemlerini simüle eden temel bir modül tanımı yapıyoruz.
// Gerçekte, burada 'device-tree' crate'inden yapılar kullanılmalıdır.

/// Aygıt Ağacı Kaynağı (Device Tree Source) adresi ve boyutu.
#[derive(Debug, Clone, Copy)]
pub struct DtbInfo {
    /// Fiziksel bellek adresi (PPC/ARM'de genellikle böyledir)
    pub physical_address: usize,
    /// Ayrıştırılmış yapıdan çıkarılan temel donanım yapılandırması.
    pub config: Option<HardwareConfig>,
}

/// Aygıt Ağacından okunan temel donanım yapılandırmaları.
#[derive(Debug, Clone, Copy)]
pub struct HardwareConfig {
    /// Seri Port (UART) MMIO veya IO port adresi
    pub console_addr: usize,
    /// Başlangıç Fiziksel Bellek adresi
    pub ram_start: usize,
    /// Toplam Bellek Boyutu (bayt)
    pub ram_size: usize,
    /// Kesme Kontrolcüsü (APIC/HPET/PCIe) adresi (Temsili)
    pub interrupt_controller_addr: usize,
}

/// Aygıt Ağacını ayrıştırmaktan sorumlu statik yapı.
pub struct DtbParser {
    _phantom: PhantomData<()>,
}

// DTB'nin zaten ayrıştırılıp ayrıştırılmadığını izler (Atomik Bayrak)
static mut DTB_INFO: DtbInfo = DtbInfo {
    physical_address: 0,
    config: None,
};

impl DtbParser {
    /// Önyükleyici tarafından çekirdeğe iletilen FDT adresini kaydeder.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon çekirdek başlatma aşamasında bir kez çağrılmalıdır.
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
        // FDT'nin başlangıcını kontrol et (Magic number kontrolü, 0xD00DFEED)
        // Burada, gerçek 'device-tree' kütüphanesi çağrılacaktır.

        // Aşağıdaki kod, kütüphane çağrıldığında dönecek olan
        // varsayımsal bir HardwareConfig yapısını simüle etmektedir.
        
        // Simülasyon: AMD64 platformunda tipik olarak ACPI/BIOS/UEFI
        // bilgileri DTB'ye çevrilmiştir (veya DTB sadece ek donanımlar içindir).
        let config = HardwareConfig {
            // Seri Port COM1 adresi (IO Port, MMIO değil)
            // DTB'de bu, 'compatible = "pci-serial..." ' veya benzeri bir düğümden okunur.
            console_addr: 0x3F8, // Varsayılan COM1 IO Port Adresi
            
            // Bellek bilgisi (genellikle 'memory' düğümünden alınır)
            ram_start: 0x1000_0000, // Varsayılan başlangıç adresi
            ram_size: 512 * 1024 * 1024, // 512MB
            
            // Kesme Kontrolcüsü adresi (APIC MMIO adresi veya HPET)
            interrupt_controller_addr: 0xFEE0_0000, // Varsayılan yerel APIC MMIO
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
        serial_println!("[DTB] DTB adresi bilinmiyor (0x0).");
        return;
    }
    
    // Güvenlik: Adresin geçerli bir bellek aralığında olduğunu varsayıyoruz.
    let header_slice = unsafe { 
        slice::from_raw_parts(dtb_addr as *const u32, 4) 
    };

    // Bellek erişimi için MMU'nun zaten ayarlanmış olması GEREKİR.
    // Başlangıç adresinde DTB'nin varlığını doğrular (Magic, Size, Version).
    
    // Rust'ta Big Endian (dtb formatı) okuması için özel işleme ihtiyacımız var.
    // Geçici olarak bu işlemi atlayıp sadece ham değeri basıyoruz.
    let magic = u32::from_be(header_slice[0]);

    // serial_println! macro'sunun arch/amd64/console.rs'te tanımlandığını varsayıyoruz.
    serial_println!("[DTB] DTB Adresi: {:#x}", dtb_addr);
    serial_println!("[DTB] Magic (Beklenen 0xd00dfeed): {:#x}", magic);
    
    // Ayrıştırılmış yapılandırma varsa onu da bas
    if let Ok(config) = DtbParser::get_config() {
        serial_println!("[DTB] -> Konsol Adresi: {:#x}", config.console_addr);
        serial_println!("[DTB] -> RAM Boyutu: {} MB", config.ram_size / 1024 / 1024);
    } else {
        serial_println!("[DTB] -> Yapılandırma henüz ayrıştırılmadı.");
    }
}
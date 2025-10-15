#![allow(dead_code)]

// Diğer modüllere olan bağımlılıkları içeri aktaralım
use crate::platform::{Platform, PlatformManager};
use crate::platformgeneric::{KernelError, SystemConstants};
use core::ops::RangeInclusive;

/// Bellek Yönetimi için Ortak Arayüz (Trait).
///
/// Tüm platformlar bu trait'i uygulayarak temel bellek hizmetlerini sağlar.
pub trait MemoryManager {
    /// İşletim sistemi başlangıcında sayfalama (paging) ve bellek korumasını kurar.
    /// Statik olarak tahsis edilmiş tüm bölgeleri (çekirdek kodu/veri, görev yığınları) haritalar.
    ///
    /// # Güvenlik Notu
    /// Yalnızca bir kez ve tek çekirdekli başlatma aşamasında çağrılmalıdır.
    fn initialize_memory_protection() -> Result<(), KernelError>;

    /// Bir sanal adresi karşılık gelen fiziksel adrese çevirir.
    /// Statik haritalama olduğu için bu işlem deterministik olmalıdır.
    fn translate_virtual_to_physical(virtual_addr: usize) -> Option<usize>;

    /// Verilen sanal adrese sahip bir bellek bölgesinin erişim haklarını günceller.
    /// Sert Gerçek Zamanlı sistemde bu, görevler arası izolasyon için önemlidir.
    fn set_access_permissions(virtual_addr: usize, size: usize, read: bool, write: bool, execute: bool) -> Result<(), KernelError>;
}

// -----------------------------------------------------------------------------
// STATİK BELLEK YAPILANDIRMASI VE ADRES SABİTLERİ
// -----------------------------------------------------------------------------

/// NanoKernel'de kullanılan statik bellek bölgelerinin başlangıç/bitiş adresleri.
/// Bu adresler, bootloader ve linkleme betiği (linker script) tarafından belirlenir.
pub struct MemoryRegions;

impl MemoryRegions {
    /// Çekirdek kodunun ve salt okunur verilerin bulunduğu sanal adres aralığı.
    /// Statik bellek yönetimi için kritik bir sabittir.
    pub const KERNEL_TEXT_VADDR: RangeInclusive<usize> = 0xC000_0000..=0xC100_0000;

    /// Statik olarak tahsis edilmiş görev yığınlarının başlangıç sanal adresi.
    /// Görev yığınları ardışık olmalıdır.
    pub const TASK_STACKS_VADDR_START: usize = 0xC100_0000;
    
    /// Her görev için ayrılan yığın boyutu (örnek: 8KB). Statik boyut sabittir.
    pub const TASK_STACK_SIZE: usize = 8 * 1024; 

    /// G/Ç (I/O) cihazları için ayrılmış bellek eşlemeli bölgenin başlangıç adresi.
    pub const MMIO_VADDR_START: usize = 0xFFFFFFFF_0000_0000;
}

/// Statik Görev Yığınlarının Yönetimi için bir yapı.
/// Bu yapının amacı, görevlere statik olarak ayrılmış yığınları vermektir.
pub struct TaskStackAllocator {
    // Yığınların kullanılıp kullanılmadığını izleyen statik bir bayrak dizisi.
    // Görev sayısı kadar bayrak tutulur.
    is_allocated: [bool; SystemConstants::MAX_TASKS],
    // Atomik işlemler için spinlock (Gerekirse platformgeneric'ten alınabilir)
    // lock: platformgeneric::spinlock::Spinlock,
}

// Global Statik Yığın Ayırıcı (Sistem başlatılmadan önce kullanılabilir olmalı)
// UnsafeCell kullanılır çünkü statik, değiştirilebilir veri.
static mut GLOBAL_TASK_STACK_ALLOCATOR: TaskStackAllocator = TaskStackAllocator {
    is_allocated: [false; SystemConstants::MAX_TASKS],
};

impl TaskStackAllocator {
    /// Belirtilen görev ID'si için yığının başlangıç sanal adresini hesaplar.
    pub fn get_stack_base_address(task_id: usize) -> Option<usize> {
        if task_id >= SystemConstants::MAX_TASKS {
            return None;
        }
        
        // Statik yığın adresini deterministik olarak hesaplar:
        // Başlangıç Adresi + (Görev ID * Yığın Boyutu)
        MemoryRegions::TASK_STACKS_VADDR_START
            .checked_add(task_id * MemoryRegions::TASK_STACK_SIZE)
    }

    /// Bir görev ID'si için statik yığını tahsis eder ve başlangıç adresini döndürür.
    pub fn allocate_stack(task_id: usize) -> Result<usize, KernelError> {
        if task_id >= SystemConstants::MAX_TASKS {
            return Err(KernelError::InvalidArgument);
        }

        // --- Gerçek bir çekirdekte burada bir kilit (Spinlock) olmalıdır! ---
        unsafe {
            if GLOBAL_TASK_STACK_ALLOCATOR.is_allocated[task_id] {
                return Err(KernelError::ResourceBusy);
            }
            
            // Yığını tahsis edildi olarak işaretle
            GLOBAL_TASK_STACK_ALLOCATOR.is_allocated[task_id] = true;
        }

        // Yığın taban adresini hesapla (üst adrese yakın)
        let base_addr = Self::get_stack_base_address(task_id)
            .ok_or(KernelError::OutOfMemoryStatic)?;

        // Yığın pointerı genellikle en yüksek adresten başlar ve aşağı doğru büyür.
        Ok(base_addr + MemoryRegions::TASK_STACK_SIZE)
    }

    /// Bir görevin yığınını serbest bırakır.
    pub fn deallocate_stack(task_id: usize) -> Result<(), KernelError> {
        if task_id >= SystemConstants::MAX_TASKS {
            return Err(KernelError::InvalidArgument);
        }

        // --- Gerçek bir çekirdekte burada bir kilit (Spinlock) olmalıdır! ---
        unsafe {
            if !GLOBAL_TASK_STACK_ALLOCATOR.is_allocated[task_id] {
                return Err(KernelError::NotFound);
            }
            
            // Yığını serbest bırakıldı olarak işaretle
            GLOBAL_TASK_STACK_ALLOCATOR.is_allocated[task_id] = false;
        }
        
        // Bu noktada, bellek bölgesinin temizlenmesi veya koruma ayarlarının
        // kaldırılması mimariye özgü PlatformManager üzerinden yapılmalıdır.
        Ok(())
    }
}

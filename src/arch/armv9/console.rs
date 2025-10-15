use crate::platform::PlatformManager;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

// Genellikle Raspberry Pi gibi gelişim kartlarında kullanılan PL011 UART'ın
// veya 16550 uyumlu bir UART'ın TEMEL MMIO adresi.
// Not: Gerçek gömülü sisteminizde bu adres farklı olacaktır!
const UART_MMIO_ADDR: usize = 0xFE20_1000; // Örnek adres (Raspberry Pi 3/4)

// 16550 Uyumlu UART Yazmaçları için Ofsetler
const DATA_REGISTER_OFFSET: usize = 0x00; // Veri (TX/RX)
const LINE_STATUS_REGISTER_OFFSET: usize = 0x05; // Hat Durumu (TX Boş mu?)

/// ARMv9 MMIO UART'ı yöneten yapı.
///
/// Bu yapı statiktir ve çıktı deterministik olmalıdır.
pub struct Uart;

// UART'ın başlatılıp başlatılmadığını izler (Atomik Bayrak)
static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

impl Uart {
    /// UART MMIO adresini döndürür.
    fn get_register_addr(offset: usize) -> usize {
        // Güvenli toplama işlemi (overflow kontrolü)
        UART_MMIO_ADDR.checked_add(offset).unwrap_or_else(|| 
            // no-std ortamında panik yapma veya hata döndürme
            // Basitlik için burada unwrap kullanılmıştır.
            panic!("UART MMIO adresi taşması")
        )
    }

    /// UART'ı temel bir konfigürasyonla başlatır.
    pub fn init() {
        if IS_INITIALIZED.load(Ordering::Acquire) {
            return; // Zaten başlatılmışsa tekrar başlatma
        }
        
        // --- UART Konfigürasyonu (Basitleştirilmiş) ---
        // Gerçek bir init fonksiyonu, baud hızı, FIFO'lar vb. için birçok yazmaç yazacaktır.
        // Bu örnekte, yalnızca veri göndermeye odaklanıyoruz.
        
        // Bu işlemler, src/arch/armv9/platformmod.rs dosyasındaki
        // write_byte_to_address fonksiyonunu kullanarak yapılmalıdır.
        
        // Örn: PL011 için FCR'yi etkinleştirme (Bu adım atlanmıştır, 
        // varsayılan olarak zaten etkin olduğu varsayılır.)

        IS_INITIALIZED.store(true, Ordering::Release);
    }
    
    /// UART'ın boş olup olmadığını kontrol eder (göndermeye hazır mı?).
    ///
    /// # Güvenlik Notu
    /// MMIO okuma işlemi olduğu için `unsafe` gerektirir.
    fn is_transmit_empty() -> bool {
        // PlatformManager'ı kullanarak Hat Durumu Yazmacını oku (MMIO)
        let status = unsafe { 
            PlatformManager::read_byte_from_address(
                Self::get_register_addr(LINE_STATUS_REGISTER_OFFSET)
            ) 
        };
        
        // 16550 Uyumlu: Bit 5 (THRE - Transmitter Holding Register Empty)
        (status & 0x20) != 0 // Eğer bit 5 ayarlanmışsa (1) -> Boş
    }

    /// UART'a bir bayt yazar.
    pub fn write_byte(byte: u8) {
        // Portun boş olmasını bekle (Busy-Waiting)
        // Sert Gerçek Zamanlı sistemde determinizmi korur.
        while !Self::is_transmit_empty() {}

        // Veri Yazmacına baytı yaz
        unsafe {
            PlatformManager::write_byte_from_address(
                Self::get_register_addr(DATA_REGISTER_OFFSET), 
                byte
            )
        }
    }
}

/// `core::fmt::Write` trait'ini uygulayarak formatlı çıktıya izin verir.
impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                // `\n` (LF) gördüğünde `\r\n` (CRLF) olarak gönder
                b'\n' => {
                    self.write_byte(b'\r');
                    self.write_byte(b'\n');
                }
                // Diğer karakterleri doğrudan gönder
                _ => self.write_byte(byte),
            }
        }
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// GENEL KONSOL ÇIKTI FONKSİYONLARI (Makrolar)
// -----------------------------------------------------------------------------

/// Güvenli G/Ç için statik bir UART örneği
static mut UART_DEVICE: Uart = Uart;

/// Çekirdek içerisindeki tüm formatlı çıktı çağrılarını yakalayan makro.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        // Tam yolu kullanmak önemli
        use $crate::arch::armv9::console::Uart; 

        // Not: Gerçek bir çekirdekte, bu kısım yarış koşullarını önlemek için 
        // ya bir Spinlock ile korunmalı ya da Kesmeler devre dışı bırakılmalıdır.
        unsafe {
             let _ = write!($crate::arch::armv9::console::Uart, $($arg)*);
        }
    });
}

/// Yeni satır ekleyen çıktı makrosu.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}

// Makroların kök kütüphaneden (lib.rs) dışa aktarılması gerektiğini unutmayın.
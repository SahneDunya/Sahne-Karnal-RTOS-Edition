use core::sync::atomic::{AtomicUsize, Ordering};
use core::cell::UnsafeCell;

/// Her mesaj için maksimum sabit boyutu tanımlar.
///
/// NanoKernel'de, dinamik bellekten kaçınmak için mesajlar sabit boyutta olmalıdır.
pub const MESSAGE_DATA_SIZE: usize = 64;

/// Statik IPC mesajının yapısı.
#[derive(Clone, Copy)] // Statik veri olduğu için kopyalanabilir ve taşınabilir olmalı
pub struct IpcMessage {
    /// Mesajı gönderen görevin (Task) veya bileşenin kimliği.
    pub sender_id: u8,
    /// Mesajın türünü veya amacını belirtir (örneğin, Sürücü_A_Talebi, Cevap_B).
    pub message_type: u8,
    /// Mesajın ham veri yükü. Statik olarak belirlenmiş boyut.
    pub payload: [u8; MESSAGE_DATA_SIZE],
    /// Kullanılan gerçek veri boyutu.
    pub payload_size: u8,
}

impl Default for IpcMessage {
    /// Boş bir mesaj oluşturur.
    fn default() -> Self {
        IpcMessage {
            sender_id: 0,
            message_type: 0,
            payload: [0; MESSAGE_DATA_SIZE],
            payload_size: 0,
        }
    }
}

/// Statik boyutlu mesaj kuyruğu için maksimum derinlik.
pub const QUEUE_DEPTH: usize = 8;

/// NanoKernel'deki Görevler Arası İletişim (IPC) Kuyruğu.
///
/// Bu kuyruk statiktir, dinamik bellek kullanmaz ve atomik sayaçlarla yönetilir.
/// Güvenli erişim için kilit (Lock) mekanizması eklenmelidir (şimdilik basitleştirilmiş).
pub struct IpcQueue {
    /// Kuyruğun içindeki mesajlar. Statik olarak belirlenmiş boyut.
    messages: [UnsafeCell<IpcMessage>; QUEUE_DEPTH],
    /// Kuyruğun başına işaret eden atomik sayaç.
    head: AtomicUsize,
    /// Kuyruğun sonuna işaret eden atomik sayaç.
    tail: AtomicUsize,
}

// İletişim kuyruğunun statik olarak güvenli bir şekilde kullanılabilmesi için gerekli.
unsafe impl Sync for IpcQueue {}

impl IpcQueue {
    /// Sabit bir IPC kuyruğu örneği oluşturur (Derleme zamanı sabiti).
    /// Statik değişkenler için kullanılır.
    pub const fn new() -> Self {
        // Rust'ta sabit dizileri UnsafeCell ile başlatmanın güvenli yolu
        // Mesajlar varsayılan olarak sıfır/boş başlatılır.
        let empty_message = UnsafeCell::new(IpcMessage::default());
        IpcQueue {
            messages: [empty_message; QUEUE_DEPTH],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Kuyruğun tamamen dolu olup olmadığını kontrol eder.
    pub fn is_full(&self) -> bool {
        let next_tail = (self.tail.load(Ordering::Acquire) + 1) % QUEUE_DEPTH;
        next_tail == self.head.load(Ordering::Acquire)
    }

    /// Kuyruğun tamamen boş olup olmadığını kontrol eder.
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }

    /// Mesajı kuyruğa ekler (Gönderme).
    ///
    /// # Parametreler
    /// * `message`: Kuyruğa eklenecek mesaj.
    ///
    /// # Dönüş Değeri
    /// Başarılı ise `Ok(())`, kuyruk dolu ise `Err(IpcMessage)` (Gönderilemeyen mesaj).
    pub fn send(&self, message: IpcMessage) -> Result<(), IpcMessage> {
        // İleride buraya kilit (spinlock) mekanizması eklenmelidir!
        // Şu an sadece atomik sayaçlarla yönetiyoruz.
        
        if self.is_full() {
            return Err(message);
        }

        let tail = self.tail.load(Ordering::Acquire);

        // SAFETY: head ve tail atomik olarak güncellendiği için,
        // sadece bir göndericinin bu indekse aynı anda yazacağı varsayılır.
        // *GERÇEK* IPC için bu kısım bir kilit ile korunmalıdır.
        unsafe {
            *self.messages[tail].get() = message;
        }

        let next_tail = (tail + 1) % QUEUE_DEPTH;
        self.tail.store(next_tail, Ordering::Release);
        
        Ok(())
    }

    /// Kuyruktan bir mesaj alır (Alma).
    ///
    /// # Dönüş Değeri
    /// Kuyruk boş değilse `Some(IpcMessage)`, boş ise `None`.
    pub fn receive(&self) -> Option<IpcMessage> {
        // İleride buraya kilit (spinlock) mekanizması eklenmelidir!
        
        if self.is_empty() {
            return None;
        }

        let head = self.head.load(Ordering::Acquire);

        // SAFETY: head ve tail atomik olarak güncellendiği için,
        // sadece bir alıcının bu indeksten aynı anda okuyacağı varsayılır.
        // *GERÇEK* IPC için bu kısım bir kilit ile korunmalıdır.
        let message = unsafe {
            (*self.messages[head].get()).clone()
        };

        let next_head = (head + 1) % QUEUE_DEPTH;
        self.head.store(next_head, Ordering::Release);

        Some(message)
    }
}

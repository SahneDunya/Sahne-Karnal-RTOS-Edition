// src/arch/amd64/task.rs
// AMD64 (x86_64) mimarisine özgü görev (task) ve bağlam (context) yönetimi.

use core::arch::asm;
use crate::serial_println;

/// Görev bağlamını (task context) saklamak için kullanılan yapı.
/// Bu yapı, görev anahtarlama sırasında kurtarılması gereken tüm 
/// genel amaçlı yazmaçları içerir.
///
/// Not: Yazmaçlar, yığına tam olarak hangi sırayla kaydedilip 
/// geri yüklenecekse o sırayla tanımlanmalıdır.
#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContext {
    // Yazmaçlar: Görev anahtarlama assembly kodu tarafından kaydedilir/yüklenir.
    // GPR'lar (r15, r14, r13, r12, rbp, rbx)
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbp: u64,
    rbx: u64,
    
    // Yığın işaretçisi (Stack Pointer) - görev anahtarlamanın kalbi
    // rdi'ye parametre olarak geçirilen Context yapısının adresi bu alana yazılır.
    // rdi, rsi (kullanılmaz), rdx (kullanılmaz), rcx (kullanılmaz), r8-r11 (kullanılmaz)
    // C çağrı konvansiyonu gereği rdi, rsi, rdx, rcx, r8, r9 ilk 6 parametredir.
    // rbp ve rbx, callee-saved (çağrılan tarafından korunur) yazmaçlardır.
    // r12-r15 de callee-saved yazmaçlardır.
    rsp: u64, 
    
    // Komut işaretçisi (Instruction Pointer) - görev başladığında nereye zıplanacağını belirler.
    rip: u64, 
}

impl TaskContext {
    /// Yeni bir görev bağlamı oluşturur.
    /// 
    /// # Argümanlar
    /// * `stack_top`: Görevin yığınının en üst adresi.
    /// * `entry_point`: Görevin başlayacağı fonksiyonun adresi.
    pub fn new(stack_top: u64, entry_point: u64) -> Self {
        // Yeni bir görev başlatıldığında, görev anahtarlama assembly kodu 
        // yığından bu yazmaçları POP etmeyi bekleyecektir.
        Self {
            // Yazmaçlar sıfırlanır.
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            
            // Yeni görev için rsp, yığının üstü olarak ayarlanır.
            rsp: stack_top,
            // rip, görevin giriş noktası (fonksiyon adresi) olarak ayarlanır.
            rip: entry_point,
        }
    }

    /// Bir bağlam anahtarlama işlemi sırasında yazmaç durumunu kaydetmek ve 
    /// yeni görevden yazmaç durumunu yüklemek için kullanılan harici assembly fonksiyonu.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, C ABI'sine uymalıdır: `fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext)`.
    ///
    /// # Güvenilirlik
    /// `switch_context`'in doğru çalışması, `TaskContext` yapısının düzenine
    /// ve assembly kodunun yığın üzerindeki yazmaçları doğru sırayla kaydetmesine/yüklemesine bağlıdır.
    #[inline(always)]
    pub unsafe fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext) {
        serial_println!("[TASK] Bağlam Anahtarlama: {:#x} -> {:#x}", 
                        old_context as u64, new_context as u64);

        // Bu fonksiyon, Rust'ta tanımlanmamış, ancak harici bir assembly bloğu veya 
        // ayrı bir assembly dosyası tarafından sağlanacak olan çekirdek anahtarlama rutinine zıplar.
        // Rust'ta inline assembly ile yazılım anahtarlama örneği:
        asm!(
            // --------------------- Mevcut Görevin Durumunu Kaydet ---------------------
            // rdi: old_context, rsi: new_context (C ABI)
            
            // 1. Callee-Saved (çağrılan tarafından korunan) GPR'ları yığına kaydet.
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push rbp",
            "push rbx",
            
            // 2. Mevcut RSP'yi (yığın işaretçisi) TaskContext.rsp alanına kaydet.
            // MOV RDI, RSP
            "mov [rdi + 48], rsp", // 48: rsp alanının TaskContext'teki offset'i (6 yazmaç * 8 bayt = 48)
            
            // 3. Mevcut RIP'i (dönüş adresini) TaskContext.rip alanına kaydet.
            // Bu, `call` talimatı tarafından yığına atılan adrestir. Bu adresi 
            // manuel olarak bulmak zordur, bu nedenle bu kodda RIP'i doğru kaydetmek için 
            // genellikle yığın üzerinde ek işlemler gerekir veya bu fonksiyonun 
            // başlangıcındaki dönüş adresi okunmalıdır. Basitlik için bu adım ihmal edilebilir 
            // veya anahtarlama işlemi doğrudan yığının kendisinden başlatılmalıdır.
            
            // Basitleştirilmiş: Sadece RSP'yi kaydetmek, dönüş için yeterlidir (RIP yığında kalır).
            // Ancak TaskContext yapımız RIP içerdiği için, bu anahtarlamanın 
            // yığın üzerinde gerçekleşen bir 'zıplama' olduğunu varsayıyoruz.

            // --------------------- Yeni Görevin Durumunu Yükle ---------------------
            // rsi: new_context
            
            // 1. Yeni RSP'yi yükle
            // MOV RSP, [RSI + 48]
            "mov rsp, [rsi + 48]", 
            
            // 2. Callee-Saved GPR'ları TaskContext'ten yükle
            // POP RDX, POP RBP, ...
            "pop rbx",
            "pop rbp",
            "pop r12",
            "pop r13",
            "pop r14",
            "pop r15",
            
            // 3. Yeni görevin giriş noktasına zıpla (RIP yüklemesi)
            // RIP'i yüklemek için TaskContext.rip adresi okunup zıplanır.
            // Assembly'de bu, genellikle `ret` veya `jmp` ile yapılır.
            // TaskContext'in 56 offset'te RIP tuttuğunu varsayalım (48 + 8 = 56).
            "mov rax, [rsi + 56]", // rax = new_context->rip
            "jmp rax",
            
            // Bu inline assembly bloğu, tüm anahtarlama görevini yapar.
            in("rdi") old_context,
            in("rsi") new_context,
            // rdx, rcx, r8, r9'u kullanmıyoruz, ancak C ABI'sinde bunlar caller-saved.
            options(noreturn, preserves_flags)
        );
    }
}


// -----------------------------------------------------------------------------
// Görev Başlatma (Task Entry)
// -----------------------------------------------------------------------------

/// Yeni görevlerin ilk başladığı yer. 
/// Görev, bu fonksiyonun sonundan asla dönmemelidir (return).
///
/// # Argümanlar
/// * `arg`: Göreve geçirilen u64 formatında argüman.
/// * `func`: Görevin gerçek giriş noktası (başlatılacak fonksiyon).
extern "C" fn task_entry(func: u64, arg: u64) -> ! {
    serial_println!("[TASK] Yeni Görev Başlatılıyor. Argüman: {:#x}", arg);

    // Fonksiyon işaretçisini (u64) gerçek fonksiyona dönüştür.
    let entry_func: fn(u64) = unsafe { 
        core::mem::transmute(func as *const ()) 
    };

    // Gerçek görev fonksiyonunu çağır
    entry_func(arg);

    // Görev tamamlandı. Normalde bu noktaya gelinmemelidir.
    serial_println!("[TASK] Görev Tamamlandı! Çekirdek Kapatma İsteği Gönderiliyor.");
    
    // Görev tamamlandığında, çekirdek durdurulmalıdır.
    // Bu, çekirdek ana iş parçacığı tarafından temizlenmeli veya 
    // özel bir "Exit" system call'u çağrılmalıdır.
    loop {
        unsafe {
            io::hlt();
        }
    }
}


// -----------------------------------------------------------------------------
// Deneme/Başlatma Fonksiyonu
// -----------------------------------------------------------------------------

/// Deneme amacıyla statik bir görev başlatma fonksiyonu.
pub fn initialize_tasking() {
    serial_println!("[TASK] AMD64 Görev Modülü Başlatılıyor...");
    
    // Örnek: Görev giriş noktasının adresi
    let entry_point_addr = task_entry as *const (); 
    
    serial_println!("[TASK] Task Entry Adresi: {:#x}", entry_point_addr as u64);
    
    // NOT: Gerçek bir çekirdek, bu TaskContext'leri bellekten ayırır 
    // ve switch_context'i kullanarak görevleri yönetir.
}
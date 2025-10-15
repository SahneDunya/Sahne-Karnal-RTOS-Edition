// src/arch/armv9/task.rs
// ARMv9 (aarch64) mimarisine özgü görev (task) ve bağlam (context) yönetimi.

use core::arch::asm;
use crate::serial_println;

/// Görev bağlamını (task context) saklamak için kullanılan yapı.
/// Bu yapı, görev anahtarlama sırasında kurtarılması gereken tüm 
/// **Callee-Saved** (çağrılan tarafından korunan) yazmaçları içerir.
///
/// ARMv9'da yazılımsal bağlam anahtarlama için genellikle x19'dan x30'a (LR) 
/// kadar olan yazmaçlar ve SP kaydedilir.
#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContext {
    // Callee-Saved Yazmaçlar (x19 - x30) - 12 adet
    // x19'dan x28'e kadar GPR'lar (10 adet)
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    
    // FP (Frame Pointer) x29 ve LR (Link Register) x30
    x29: u64, 
    x30: u64, // LR (Link Register)
    
    // Yığın işaretçisi (Stack Pointer) - Bağlam anahtarlamanın kalbi
    sp: u64, 
    
    // Komut işaretçisi (Instruction Pointer) - Yeni görevin başladığı yer
    // Bu, anahtarlama fonksiyonundan dönüş adresi olarak kullanılır.
    pc: u64, 
}

impl TaskContext {
    /// Yeni bir görev bağlamı oluşturur.
    /// 
    /// # Argümanlar
    /// * `stack_top`: Görevin yığınının en üst adresi.
    /// * `entry_point`: Görevin başlayacağı fonksiyonun adresi.
    pub fn new(stack_top: u64, entry_point: u64) -> Self {
        // Yeni bir görev başlatıldığında, anahtarlama kodu bu yapıyı yükler.
        Self {
            // Callee-Saved yazmaçlar sıfırlanır.
            ..Default::default()
            
            // sp, yeni görevin yığınının üstü olarak ayarlanır.
            sp: stack_top,
            
            // pc, görevin giriş noktası (fonksiyon adresi) olarak ayarlanır.
            pc: entry_point,
            
            // x30 (LR) da aslında entry_point olarak ayarlanabilir, 
            // böylece anahtarlama fonksiyonundan `ret` ile dönülür.
            x30: entry_point,
        }
    }

    /// Bir bağlam anahtarlama işlemi sırasında yazmaç durumunu kaydetmek ve 
    /// yeni görevden yazmaç durumunu yüklemek için kullanılan harici assembly fonksiyonu.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, C ABI'sine uymalıdır: `fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext)`.
    /// Argümanlar x0 ve x1'e geçirilir.
    #[inline(always)]
    pub unsafe fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext) {
        serial_println!("[TASK] Bağlam Anahtarlama: {:#x} -> {:#x}", 
                        old_context as u64, new_context as u64);

        asm!(
            // --------------------- Mevcut Görevin Durumunu Kaydet ---------------------
            // x0: old_context, x1: new_context (C ABI)
            
            // 1. Callee-Saved GPR'ları TaskContext'e kaydet.
            // x19-x28 (10 adet) ve x29/x30 (2 adet) = 12 yazmaç.
            // str x1, [base, #offset] (Store Register)
            
            // x19-x28'i kaydet (10 yazmaç = 80 bayt). x19 offset 0'da başlar.
            "stp x19, x20, [x0, #0]",  
            "stp x21, x22, [x0, #16]",
            "stp x23, x24, [x0, #32]",
            "stp x25, x26, [x0, #48]",
            "stp x27, x28, [x0, #64]",
            
            // x29 (FP) ve x30 (LR) kaydet. x29 offset 80'de başlar.
            "stp x29, x30, [x0, #80]",
            
            // 2. Mevcut SP'yi (yığın işaretçisi) TaskContext.sp alanına kaydet. (Offset 96)
            "mov x10, sp", // sp'yi x10'a taşı
            "str x10, [x0, #96]", 

            // 3. Mevcut PC'yi (dönüş adresini) TaskContext.pc alanına kaydet. (Offset 104)
            // Anahtarlama rutinine `blr` ile çağrıldığı varsayılarak, dönüş adresi x30 (LR)'dadır.
            // Bu zaten x30 kaydıyla yapıldı, ancak PC'yi kaydetmek için ek bir alan (TaskContext.pc) var.
            // Bu alana, bu fonksiyonun dönüş adresini kaydetmek için LR'ın içeriği kullanılır.
            // NOT: Rust'tan asm'e girildiğinde LR zaten yığında olabilir. Basitleştirme için:
            "mov x10, x30",
            "str x10, [x0, #104]",
            
            // --------------------- Yeni Görevin Durumunu Yükle ---------------------
            // x1: new_context
            
            // 1. Yeni SP'yi yükle (Offset 96)
            "ldr x10, [x1, #96]", 
            "mov sp, x10", 
            
            // 2. Callee-Saved GPR'ları TaskContext'ten yükle
            // x19-x28 (Offset 0)
            "ldp x19, x20, [x1, #0]",  
            "ldp x21, x22, [x1, #16]",
            "ldp x23, x24, [x1, #32]",
            "ldp x25, x26, [x1, #48]",
            "ldp x27, x28, [x1, #64]",

            // x29 (FP) ve x30 (LR) yükle (Offset 80)
            "ldp x29, x30, [x1, #80]",
            
            // 3. Yeni görevin giriş noktasına zıpla (PC yüklemesi)
            // PC'yi yüklemek için TaskContext.pc adresi okunup zıplanır.
            // PC Offset 104
            "ldr x10, [x1, #104]", 
            "br x10", // Branch to Register (Görevi başlatır)
            
            in("x0") old_context,
            in("x1") new_context,
            // x2-x18 caller-saved, x19-x30 callee-saved
            out("x10") _, // x10 geçici olarak kullanıldı
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
/// * `func`: Görevin gerçek giriş noktası (başlatılacak fonksiyonun adresi, x0'da olmalı).
/// * `arg`: Göreve geçirilen u64 formatında argüman (x1'de olmalı).
extern "C" fn task_entry(func: u64, arg: u64) -> ! {
    serial_println!("[TASK] Yeni Görev Başlatılıyor. Argüman: {:#x}", arg);

    // Fonksiyon işaretçisini (u64) gerçek fonksiyona dönüştür.
    // ARMv9 C ABI'sinde ilk argüman x0'da beklenir, bu da func'a karşılık gelir.
    let entry_func: fn(u64) = unsafe { 
        core::mem::transmute(func as *const ()) 
    };

    // Gerçek görev fonksiyonunu çağır (arg, x1'de beklenir, bu da arg'a karşılık gelir)
    entry_func(arg);

    // Görev tamamlandı. Normalde bu noktaya gelinmemelidir.
    serial_println!("[TASK] Görev Tamamlandı! İşlemci durduruluyor.");
    
    // Görev tamamlandığında, çekirdek durdurulmalıdır.
    loop {
        unsafe {
            io::wfi(); // Wait For Interrupt
        }
    }
}


// -----------------------------------------------------------------------------
// Deneme/Başlatma Fonksiyonu
// -----------------------------------------------------------------------------

/// Deneme amacıyla statik bir görev başlatma fonksiyonu.
pub fn initialize_tasking() {
    serial_println!("[TASK] ARMv9 Görev Modülü Başlatılıyor...");
    
    // Örnek: Görev giriş noktasının adresi
    let entry_point_addr = task_entry as *const (); 
    
    serial_println!("[TASK] Task Entry Adresi: {:#x}", entry_point_addr as u64);
}
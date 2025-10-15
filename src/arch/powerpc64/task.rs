// src/arch/powerpc64/task.rs
// PowerPC 64 (PPC64) mimarisine özgü görev (task) ve bağlam (context) yönetimi.

use core::arch::asm;
use crate::serial_println;

/// Görev bağlamını (task context) saklamak için kullanılan yapı.
/// PPC64 ABI'de Callee-Saved yazmaçlar: r14-r31 (18 GPR), r2 (TOC), r13 (TP), 
/// CR (Condition Register), LR (Link Register).
#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContext {
    // Callee-Saved GPR'lar (r14 - r31) - 18 adet
    r14: u64,
    r15: u64,
    r16: u64,
    r17: u64,
    r18: u64,
    r19: u64,
    r20: u64,
    r21: u64,
    r22: u64,
    r23: u64,
    r24: u64,
    r25: u64,
    r26: u64,
    r27: u64,
    r28: u64,
    r29: u64,
    r30: u64,
    r31: u64,
    
    // Özel Callee-Saved GPR'lar
    r2_toc: u64,  // TOC (Table of Contents / RTOC)
    r13_tp: u64,  // TP (Thread Pointer)
    
    // Özel Yazmaçlar
    cr: u64,      // CR (Condition Register)
    lr: u64,      // LR (Link Register) - Geri dönüş adresi
    
    // Yığın İşaretçisi (r1) - Bağlam anahtarlamanın kalbi
    r1_sp: u64,   // SP (Stack Pointer)
    
    // Program Sayacı (r1'e geri dönüş adresi olarak kullanılır)
    pc: u64,      // PC - Görevin başlayacağı adres (genellikle LR ile aynıdır)
}

impl TaskContext {
    /// Yeni bir görev bağlamı oluşturur.
    /// 
    /// # Argümanlar
    /// * `stack_top`: Görevin yığınının en üst adresi.
    /// * `entry_point`: Görevin başlayacağı fonksiyonun adresi.
    pub fn new(stack_top: u64, entry_point: u64, rtoc: u64, thread_ptr: u64) -> Self {
        // PPC'de TOC ve TP'nin ayarlanması zorunludur.
        Self {
            ..Default::default()
            
            // Yığın işaretçisi ayarlanır
            r1_sp: stack_top,
            
            // Link Register/PC, görevin ilk başlayacağı adres olarak ayarlanır.
            lr: entry_point,
            pc: entry_point, 
            
            // TOC ve TP (Thread Pointer) değerleri ayarlanır.
            r2_toc: rtoc,
            r13_tp: thread_ptr, 
        }
    }

    /// Bir bağlam anahtarlama işlemi sırasında yazmaç durumunu kaydetmek ve 
    /// yeni görevden yazmaç durumunu yüklemek için kullanılan harici assembly fonksiyonu.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, C ABI'sine uymalıdır: `fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext)`.
    /// Argümanlar r3 ve r4'e (a0 ve a1) geçirilir.
    #[inline(always)]
    pub unsafe fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext) {
        serial_println!("[TASK] Bağlam Anahtarlama: {:#x} -> {:#x}", 
                        old_context as u64, new_context as u64);

        asm!(
            // --------------------- Mevcut Görevin Durumunu Kaydet ---------------------
            // r3: old_context, r4: new_context (C ABI'de a0 ve a1)
            
            // 1. Callee-Saved GPR'ları TaskContext'e kaydet (r14-r31)
            // std rN, offset(r3) (Store Doubleword)
            
            // r14 offset 0'da başlar. 18 GPR = 144 bayt.
            "std r14, 0(r3)",
            "std r15, 8(r3)",
            // ... r16'dan r31'e kadar (16 tanesi atlandı)
            "std r31, 136(r3)", // r31 son GPR.

            // 2. Özel Callee-Saved GPR'ları kaydet
            // r2 (TOC) offset 144
            "std r2, 144(r3)",
            // r13 (TP) offset 152
            "std r13, 152(r3)",
            
            // 3. CR ve LR yazmaçlarını kaydet (mfspr talimatı ile)
            // LR offset 160, CR offset 168
            "mflr r5",         // Move from Link Register (r5 geçici)
            "std r5, 160(r3)", // LR
            "mfcr r5",         // Move from Condition Register
            "std r5, 168(r3)", // CR
            
            // 4. r1 (SP) kaydet. Offset 176
            "std r1, 176(r3)",
            
            // 5. PC'yi kaydet (Genellikle LR'dır, burada basitleştirildi)
            "std r5, 184(r3)", // PC (LR'dan kopyalanan r5, tekrar kullanıldı)

            // --------------------- Yeni Görevin Durumunu Yükle ---------------------
            // r4: new_context
            
            // 1. Yeni r1 (SP) yükle (Offset 176)
            "ld r1, 176(r4)",  
            
            // 2. Callee-Saved GPR'ları yükle (r14-r31) (Offset 0)
            "ld r14, 0(r4)", 
            "ld r15, 8(r4)",
            // ... r16'dan r31'e kadar (16 tanesi atlandı)
            "ld r31, 136(r4)",
            
            // 3. Özel Callee-Saved GPR'ları yükle
            // r2 (TOC) offset 144
            "ld r2, 144(r4)",
            // r13 (TP) offset 152
            "ld r13, 152(r4)",
            
            // 4. CR ve LR yazmaçlarını yükle (mtspr talimatı ile)
            // LR offset 160, CR offset 168
            "ld r5, 160(r4)",  // LR
            "mtlr r5",         // Move to Link Register
            "ld r5, 168(r4)",  // CR
            "mtcrf 0xff, r5",  // Move to Condition Register Field (tüm alanlar)

            // 5. Yeni görevin PC'sine zıpla (PC offset 184)
            "ld r5, 184(r4)",
            "mtctr r5",        // Move to Count Register
            "bctr",            // Branch to Count Register (Görevi başlatır/devam ettirir)
            
            in("r3") old_context,
            in("r4") new_context,
            // r5 geçici olarak kullanıldı
            out("r5") _,
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
/// * `func`: Görevin gerçek giriş noktası (başlatılacak fonksiyonun adresi, r3'te olmalı).
/// * `arg`: Göreve geçirilen u64 formatında argüman (r4'te olmalı).
extern "C" fn task_entry(func: u64, arg: u64) -> ! {
    serial_println!("[TASK] Yeni Görev Başlatılıyor. Argüman: {:#x}", arg);

    // Fonksiyon işaretçisini (u64) gerçek fonksiyona dönüştür.
    // PPC64 C ABI'sinde ilk argüman r3 (a0)'ta beklenir.
    let entry_func: fn(u64) = unsafe { 
        core::mem::transmute(func as *const ()) 
    };

    // Gerçek görev fonksiyonunu çağır (arg, r4 (a1)'da beklenir)
    entry_func(arg);

    // Görev tamamlandı. Normalde bu noktaya gelinmemelidir.
    serial_println!("[TASK] Görev Tamamlandı! İşlemci durduruluyor.");
    
    // Görev tamamlandığında, çekirdek durdurulmalıdır.
    loop {
        unsafe {
            io::wait(); // PowerPC'de bekleme talimatı
        }
    }
}


// -----------------------------------------------------------------------------
// Deneme/Başlatma Fonksiyonu
// -----------------------------------------------------------------------------

/// Deneme amacıyla statik bir görev başlatma fonksiyonu.
pub fn initialize_tasking() {
    serial_println!("[TASK] PowerPC 64 Görev Modülü Başlatılıyor...");
    
    // Örnek: Görev giriş noktasının adresi
    let entry_point_addr = task_entry as *const (); 
    
    serial_println!("[TASK] Task Entry Adresi: {:#x}", entry_point_addr as u64);
}
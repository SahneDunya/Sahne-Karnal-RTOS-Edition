// src/arch/sparcv9/task.rs
// SPARC V9 (UltraSPARC) mimarisine özgü görev (task) ve bağlam (context) yönetimi.

use core::arch::asm;
use crate::serial_println;

/// Görev bağlamını (task context) saklamak için kullanılan yapı.
/// SPARC V9'da yazılımsal bağlam anahtarlama için kaydedilmesi gereken minimum durum:
/// Global yazmaçlar (g1-g7), Yığın İşaretçisi (r_sp), Link Register (r_lr),
/// Program Sayacı (pc), İleri Program Sayacı (npc) ve Durum Yazmaçları.
#[repr(C)]
#[derive(Debug, Default)]
pub struct TaskContext {
    // Global GPR'lar (g1 - g7) - 7 adet
    r_g1: u64,
    r_g2: u64,
    r_g3: u64,
    r_g4: u64,
    r_g5: u64,
    r_g6: u64,
    r_g7: u64,
    
    // Yığın İşaretçisi (Stack Pointer) - %sp / %o6
    r_sp: u64, // r_o6
    
    // Link Register (Return Address) - %o7
    r_lr: u64, // r_o7
    
    // Program Sayacı ve İleri Program Sayacı (görev başlama noktası)
    r_pc: u64, 
    r_npc: u64, // pc + 4
    
    // Özel Durum Yazmaçları:
    // PSR (Processor State Register) veya PSTATE (Yeni SPARC'larda)
    // Y (yazmacı), CCR (Condition Code Register)
    r_y: u64,
    r_ccr: u64, 
    
    // CWP (Current Window Pointer) veya GSW (Global Status Word) gibi özel yazmaçlar
    // Not: Gerçek bir çekirdek bu yazmaçları da kaydetmelidir.
}

impl TaskContext {
    /// Yeni bir görev bağlamı oluşturur.
    /// 
    /// # Argümanlar
    /// * `stack_top`: Görevin yığınının en üst adresi.
    /// * `entry_point`: Görevin başlayacağı fonksiyonun adresi.
    pub fn new(stack_top: u64, entry_point: u64) -> Self {
        Self {
            ..Default::default()
            
            // r_sp (%o6), yığının üstü olarak ayarlanır.
            r_sp: stack_top,
            
            // r_lr (%o7) ve PC/nPC, görevin ilk başlayacağı adres olarak ayarlanır.
            // SPARC'ta dallanmalar gecikmeli dallanma olduğu için nPC de kritiktir.
            r_lr: entry_point, 
            r_pc: entry_point,
            r_npc: entry_point.wrapping_add(4), // PC + 4
        }
    }

    /// Bir bağlam anahtarlama işlemi sırasında yazmaç durumunu kaydetmek ve 
    /// yeni görevden yazmaç durumunu yüklemek için kullanılan harici assembly fonksiyonu.
    ///
    /// # Güvenlik Notu
    /// Bu fonksiyon, C ABI'sine uymalıdır: `fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext)`.
    /// Argümanlar %o0 ve %o1'e (r24 ve r25) geçirilir.
    #[inline(always)]
    pub unsafe fn switch_context(old_context: *mut TaskContext, new_context: *const TaskContext) {
        serial_println!("[TASK] Bağlam Anahtarlama: {:#x} -> {:#x}", 
                        old_context as u64, new_context as u64);

        // Not: SPARC'ta pencere anahtarlama (save/restore) görevden önce/sonra yapılmalıdır.
        // Bu, çekirdek yazmaçlarını korumak için `save` veya `restore` talimatının çağrılmasını gerektirir.
        
        asm!(
            // SPARC'ın Yazmaç Penceresi mimarisi nedeniyle, Rust/C fonksiyonları
            // genellikle yazmaçları yığına kaydetmek için save/restore kullanır.
            // Bu switch, mevcut görevden bir SAVE ve yeni göreve bir RESTORE gibi davranmalıdır.
            
            // --------------------- Mevcut Görevin Durumunu Kaydet ---------------------
            // %o0 (r24): old_context, %o1 (r25): new_context
            
            // 1. Global GPR'ları TaskContext'e kaydet (g1-g7)
            // std rN, [r_base + offset] (Store Doubleword)
            // g1-g7, r1-r7'dir.
            "std %g1, [r24 + 0]",
            "std %g2, [r24 + 8]",
            // ... r_g3'den r_g7'ye kadar (atlandı)
            "std %g7, [r24 + 48]", // g7 offset 48

            // 2. SP (%o6) ve LR (%o7) kaydet.
            "std %o6, [r24 + 56]", // r_sp (o6)
            "std %o7, [r24 + 64]", // r_lr (o7)

            // 3. Özel Yazmaçları kaydet (PC, nPC, Y, CCR)
            // mfsr talimatı ile PSR (veya PSTATE), Y, CCR okunur.
            "rd %pc, %l0",         // r_pc'yi l0'a oku
            "rd %npc, %l1",        // r_npc'yi l1'e oku
            "std %l0, [r24 + 72]", // r_pc
            "std %l1, [r24 + 80]", // r_npc
            
            "rd %y, %l0",          // Y yazmacı
            "wr %l0, 0x1000, %asr24", // CCR (ASR24'te tutulur)
            "rd %asr24, %l1",
            "std %l0, [r24 + 88]", // r_y
            "std %l1, [r24 + 96]", // r_ccr

            // --------------------- Yeni Görevin Durumunu Yükle ---------------------
            // %o1 (r25): new_context
            
            // 1. Özel Yazmaçları yükle (Y, CCR)
            "ldd [r25 + 88], %l0", // r_y
            "wry %l0",
            "ldd [r25 + 96], %l0", // r_ccr
            "wr %l0, 0, %asr24",   // CCR

            // 2. Global GPR'ları yükle (g1-g7)
            "ldd [r25 + 0], %g1",
            "ldd [r25 + 8], %g2",
            // ... r_g3'den r_g7'ye kadar (atlandı)
            "ldd [r25 + 48], %g7",
            
            // 3. Yeni SP (%o6) ve LR (%o7) yükle.
            "ldd [r25 + 56], %o6", // r_sp (o6)
            "ldd [r25 + 64], %o7", // r_lr (o7)
            
            // 4. Yeni görevin PC/nPC'sine zıpla
            // PC/nPC'yi özel yazmaçlara yükle
            "ldd [r25 + 72], %l0", // r_pc
            "ldd [r25 + 80], %l1", // r_npc
            "jmpl %l0, %g0",       // Jump to PC (%l0)
            " nop",                // Gecikme Yuvası (nPC'yi doldurur)
            
            in("r24") old_context,
            in("r25") new_context,
            // r26-r31 (o2-o7) caller-saved
            out("r8") _, out("r9") _, // l0 ve l1 geçici olarak kullanıldı
            options(noreturn, preserves_flags)
        );
        // Yazmaç Penceresi Yönetimi: Başarılı bir anahtarlamada buraya dönülmez.
        // Geri dönülürse, bir yazmaç penceresi tutarsızlığı yaşanmış demektir.
    }
}


// -----------------------------------------------------------------------------
// Görev Başlatma (Task Entry)
// -----------------------------------------------------------------------------

/// Yeni görevlerin ilk başladığı yer. 
/// Görev, bu fonksiyonun sonundan asla dönmemelidir (return).
///
/// # Argümanlar
/// * `func`: Görevin gerçek giriş noktası (başlatılacak fonksiyonun adresi, %o0'da olmalı).
/// * `arg`: Göreve geçirilen u64 formatında argüman (%o1'de olmalı).
extern "C" fn task_entry(func: u64, arg: u64) -> ! {
    serial_println!("[TASK] Yeni Görev Başlatılıyor. Argüman: {:#x}", arg);

    // Fonksiyon işaretçisini (u64) gerçek fonksiyona dönüştür.
    // SPARC C ABI'sinde ilk argüman %o0 (r24)'te beklenir.
    let entry_func: fn(u64) = unsafe { 
        core::mem::transmute(func as *const ()) 
    };

    // Gerçek görev fonksiyonunu çağır (arg, %o1'de beklenir)
    entry_func(arg);

    // Görev tamamlandı. Normalde bu noktaya gelinmemelidir.
    serial_println!("[TASK] Görev Tamamlandı! İşlemci durduruluyor.");
    
    // Görev tamamlandığında, çekirdek durdurulmalıdır.
    loop {
        unsafe {
            io::idle(); // SPARC'ta bekleme talimatı
        }
    }
}


// -----------------------------------------------------------------------------
// Deneme/Başlatma Fonksiyonu
// -----------------------------------------------------------------------------

/// Deneme amacıyla statik bir görev başlatma fonksiyonu.
pub fn initialize_tasking() {
    serial_println!("[TASK] SPARC V9 Görev Modülü Başlatılıyor...");
    
    // Örnek: Görev giriş noktasının adresi
    let entry_point_addr = task_entry as *const (); 
    
    serial_println!("[TASK] Task Entry Adresi: {:#x}", entry_point_addr as u64);
}
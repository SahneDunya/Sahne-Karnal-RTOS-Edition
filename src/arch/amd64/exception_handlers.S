.global exception_handler_divide_by_zero
exception_handler_divide_by_zero:
    pushq $0  # Hata kodu olmayan istisnalar için 0 it
    pushq $0  # İstisna vektör numarası
    // ... Tüm GPR'ları yığına kaydet ...
    call generic_exception_handler
    // ... Tüm GPR'ları yığından geri yükle ...
    add $16, %rsp # Vektör ve Hata kodunu yığından at
    iretq       # Kesmeden Dön

.global exception_handler_general_protection_fault
exception_handler_general_protection_fault:
    pushq $13  # İstisna vektör numarası
    // ... Tüm GPR'ları yığına kaydet ...
    call generic_exception_handler_with_error
    // ... GPR'ları geri yükle ...
    add $16, %rsp # Vektör ve Hata kodunu yığından at
    iretq
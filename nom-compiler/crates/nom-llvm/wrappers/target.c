/*
 * Target initialization wrappers for nom-llvm.
 *
 * The LLVM-C header Target.h defines these functions as static inline.
 * llvm-sys (and inkwell) declare them as extern, so we need to compile
 * them into an object file that provides the actual symbols.
 *
 * This is the same approach llvm-sys uses in its own wrappers/target.c.
 */

#include <llvm-c/Target.h>

/* These wrapper functions match the extern declarations in llvm-sys/src/target.rs.
   They simply call the static inline functions from the LLVM-C header. */

void LLVM_InitializeAllTargetInfos(void) {
    LLVMInitializeAllTargetInfos();
}

void LLVM_InitializeAllTargets(void) {
    LLVMInitializeAllTargets();
}

void LLVM_InitializeAllTargetMCs(void) {
    LLVMInitializeAllTargetMCs();
}

void LLVM_InitializeAllAsmPrinters(void) {
    LLVMInitializeAllAsmPrinters();
}

void LLVM_InitializeAllAsmParsers(void) {
    LLVMInitializeAllAsmParsers();
}

void LLVM_InitializeAllDisassemblers(void) {
    LLVMInitializeAllDisassemblers();
}

typedef int LLVMBool;

LLVMBool LLVM_InitializeNativeTarget(void) {
    return LLVMInitializeNativeTarget();
}

LLVMBool LLVM_InitializeNativeAsmParser(void) {
    return LLVMInitializeNativeAsmParser();
}

LLVMBool LLVM_InitializeNativeAsmPrinter(void) {
    return LLVMInitializeNativeAsmPrinter();
}

LLVMBool LLVM_InitializeNativeDisassembler(void) {
    return LLVMInitializeNativeDisassembler();
}

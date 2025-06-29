.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    # 此段为跳板中的 trap 处理代码
    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    # 当前的 sscratch 指向应用空间中的 TrapContext
    # 当前的 sp 指向应用空间中的用户栈
    # 将 sp 指向 TrapContext 的位置
    csrrw sp, sscratch, sp
    # 在应用空间中的 TrapContext 中保存当前的寄存器等信息
    sd x1, 1*8(sp)
    # 跳过 sp(x2)
    sd x3, 3*8(sp)
    # 跳过 tp(x4)
    # 保存 x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # 使用 tn 读取 csr 寄存器，并将其值保存到 TrapContext 中
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # 读取 sscratch 并将其保存在 TrapContext 中
    csrr t2, sscratch
    sd t2, 2*8(sp)

    # 加载应用加载时初始化在 TrapContext 中的用户空间 token 等信息，用于内核内存空间切换
    # 加载 kernel_satp 到 t0
    ld t0, 34*8(sp)
    # 加载 trap_handler 到 t1
    ld t1, 36*8(sp)
    # 加载内核栈顶指针 kernel_sp
    ld sp, 35*8(sp)
    # 切换到内核空间
    csrw satp, t0
    # 刷新 TLB
    sfence.vma
    # 跳转到 rust 编写的 trap_handler
    jr t1

__restore:
    # a0: *TrapContext in user space(Constant); a1: user space token
    # switch to user space
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0
    mv sp, a0
    # now sp points to TrapContext in user space, start restoring based on it
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # restore general purpose registers except x0/sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # back to user stack
    ld sp, 2*8(sp)
    sret

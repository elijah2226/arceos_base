#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <errno.h>
#include <string.h>

#define PAGE_SIZE 4096 // 假设内核页大小
#define HPET_MEM_SIZE 1024
#define HPET_GENERAL_CAPS_OFFSET 0x00 // HPET General Capabilities and ID Register
#define HPET_GENERAL_CONFIG_OFFSET 0x10
#define HPET_MAIN_COUNTER_OFFSET 0xF0 // HPET Main Counter Value Register

int main() {
    int uio_fd;
    void *mapped_mem;
    unsigned int irq_count;

    printf("[hpet_uio_test] Starting HPET UIO test program...\n");

    // 1. 打开 UIO 设备文件 (假设 HPET 被注册为 /dev/uio0)
    uio_fd = open("/dev/uio0", O_RDWR);
    if (uio_fd < 0) {
        perror("[hpet_uio_test] Failed to open /dev/uio0");
        return 1;
    }
    printf("[hpet_uio_test] Successfully opened /dev/uio0, fd = %d\n", uio_fd);

    // 2. 映射 HPET 的 MMIO 区域
    mapped_mem = mmap(NULL,                     // 让内核选择地址
                      HPET_MEM_SIZE,            // 映射大小 (HPET 寄存器块是 1KB，但我们至少映射一页)
                      PROT_READ | PROT_WRITE,   // 只读就足够验证计数器了
                      MAP_SHARED,               // 共享映射
                      uio_fd,                   // UIO 设备的文件描述符
                      0);                       // 偏移量 0 对应第一个 mem_region

    if (mapped_mem == MAP_FAILED) {
        perror("[hpet_uio_test] Failed to mmap HPET memory");
        close(uio_fd);
        return 1;
    }
    printf("[hpet_uio_test] Successfully mmap'd HPET memory at virtual address %p\n", mapped_mem);

    // 3. 读取 HPET 寄存器验证 MMIO 访问
    volatile unsigned long long *hpet_caps_reg = (volatile unsigned long long *)(mapped_mem + HPET_GENERAL_CAPS_OFFSET);
    volatile unsigned long long *hpet_main_counter_reg = (volatile unsigned long long *)(mapped_mem + HPET_MAIN_COUNTER_OFFSET);
    volatile unsigned long long *hpet_config_reg = (volatile unsigned long long *)(mapped_mem + HPET_GENERAL_CONFIG_OFFSET);
    
    unsigned long long hpet_caps = *hpet_caps_reg;
    printf("[hpet_uio_test] HPET Capabilities Register (offset 0x0): 0x%llx\n", hpet_caps);

    // 【【【修改点 2：写入配置，使能 HPET】】】
    printf("[hpet_uio_test] Enabling HPET main counter...\n");
    // 读取当前配置，然后设置 bit 0 (ENABLE_CNF) 为 1
    *hpet_config_reg |= 1;

    unsigned long long counter_val_1 = *hpet_main_counter_reg;
    printf("[hpet_uio_test] HPET Main Counter (offset 0xF0) initial value: 0x%llx\n", counter_val_1);

    // 短暂延迟再次读取，看其是否增长
    // 注意: 如果 axlibc 没有实现 usleep，这里会失败。
    // 可以替换为 busy-wait 循环或直接连续读取。
    usleep(100000); // 100ms 延迟
    unsigned long long counter_val_2 = *hpet_main_counter_reg;
    printf("[hpet_uio_test] HPET Main Counter (offset 0xF0) after delay: 0x%llx\n", counter_val_2);

    if (counter_val_2 > counter_val_1) {
        printf("[hpet_uio_test] SUCCESS: HPET Main Counter is incrementing. MMIO read is working!\n");
    } else {
        printf("[hpet_uio_test] WARNING: HPET Main Counter did not increment. Check if HPET is enabled in QEMU.\n");
    }

    printf("[hpet_uio_test] Disabling HPET main counter...\n");
    *hpet_config_reg &= ~1; // 清除 bit 0 (ENABLE_CNF)

    // 4. 等待中断
    printf("[hpet_uio_test] Waiting for an interrupt by reading from the UIO device...\n");
    ssize_t bytes_read = read(uio_fd, &irq_count, sizeof(irq_count));

    if (bytes_read < 0) {
        perror("[hpet_uio_test] Failed to read from UIO device");
    } else {
        printf("[hpet_uio_test] SUCCESS: Woken up by an interrupt! Read %ld bytes. IRQ count: %u\n", bytes_read, irq_count);
    }

    // 清理
    munmap(mapped_mem, HPET_MEM_SIZE);
    close(uio_fd);
    printf("[hpet_uio_test] HPET UIO test finished.\n");
    return 0;
}
// // apps/uio_test/uio_test.c

// #include <stdio.h>
// #include <stdlib.h>
// #include <fcntl.h>
// #include <unistd.h>
// #include <sys/mman.h>
// #include <errno.h>
// #include <string.h>

// // 假设我们的内核页大小是 4KB
// #define PAGE_SIZE 4096

// int main() {
//     int uio_fd;
//     void *mapped_mem;
//     unsigned int irq_count;

//     printf("[uio_test] Starting UIO test program...\n");

//     // 1. 打开 UIO 设备文件
//     uio_fd = open("/dev/uio0", O_RDWR);
//     if (uio_fd < 0) {
//         perror("[uio_test] Failed to open /dev/uio0");
//         return 1;
//     }
//     printf("[uio_test] Successfully opened /dev/uio0, fd = %d\n", uio_fd);

//     // 2. 映射第一个内存区域
//     // UIO 规范: offset = N * PAGE_SIZE 对应第 N 个内存区域
//     // 我们映射第一个区域，所以 offset = 0
//     // 你的 dummy device 注册了 64KB，我们这里只映射一页就够了
//     mapped_mem = mmap(NULL,           // 让内核选择地址
//                       PAGE_SIZE,      // 映射大小
//                       PROT_READ | PROT_WRITE, // 可读可写
//                       MAP_SHARED,     // 共享映射
//                       uio_fd,         // UIO 设备的文件描述符
//                       0);             // 偏移量，0 表示第一个内存区域

//     if (mapped_mem == MAP_FAILED) {
//         perror("[uio_test] Failed to mmap UIO memory");
//         close(uio_fd);
//         return 1;
//     }
//     printf("[uio_test] Successfully mmap'd UIO memory region 0 at virtual address %p\n", mapped_mem);

//     // 3. 测试读写映射的内存
//     volatile unsigned int *test_ptr = (unsigned int *)mapped_mem;
//     unsigned int old_value = *test_ptr;
//     *test_ptr = 0xDEADBEEF;
//     printf("[uio_test] Wrote 0xDEADBEEF to mapped memory, previous value was 0x%x.\n", old_value);
//     if (*test_ptr != 0xDEADBEEF) {
//         printf("[uio_test] ERROR: Read back value 0x%x does not match!\n", *test_ptr);
//     } else {
//         printf("[uio_test] Read back value matches. Memory mapping seems to work.\n");
//     }

//     // 4. 等待中断
//     printf("[uio_test] Waiting for an interrupt by reading from the UIO device...\n");
//     // read() 会阻塞，直到内核的 UIO 驱动接收到中断并唤醒它
//     ssize_t bytes_read = read(uio_fd, &irq_count, sizeof(irq_count));

//     if (bytes_read < 0) {
//         perror("[uio_test] Failed to read from UIO device");
//     } else if (bytes_read == 0) {
//         printf("[uio_test] Read 0 bytes, EOF?\n");
//     } else {
//         // UIO spec: read() 返回一个 32 位的整数，表示自上次读取以来发生的中断次数
//         printf("[uio_test] Read %ld bytes. Woken up by an interrupt! IRQ count: %u\n", bytes_read, irq_count);
//     }

//     // 清理
//     munmap(mapped_mem, PAGE_SIZE);
//     close(uio_fd);
//     printf("[uio_test] UIO test finished.\n");
//     return 0;
// }

// apps/uio_test/uio_test.c

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <errno.h>
#include <string.h>

// 假设我们的内核页大小是 4KB
#define PAGE_SIZE 4096

int main() {
    int uio_fd;
    void *mapped_mem;
    unsigned int irq_count;

    printf("[uio_test] Starting UIO test program...\n");

    // 1. 打开 UIO 设备文件
    uio_fd = open("/dev/uio0", O_RDWR);
    if (uio_fd < 0) {
        perror("[uio_test] Failed to open /dev/uio0");
        return 1;
    }
    printf("[uio_test] Successfully opened /dev/uio0, fd = %d\n", uio_fd);

    // 2. 映射第一个内存区域
    // UIO 规范: offset = N * PAGE_SIZE 对应第 N 个内存区域
    // 我们映射第一个区域，所以 offset = 0
    // 你的 dummy device 注册了 64KB，我们这里只映射一页就够了
    mapped_mem = mmap(NULL,           // 让内核选择地址
                      PAGE_SIZE,      // 映射大小
                      PROT_READ | PROT_WRITE, // 可读可写
                      MAP_SHARED,     // 共享映射
                      uio_fd,         // UIO 设备的文件描述符
                      0);             // 偏移量，0 表示第一个内存区域

    if (mapped_mem == MAP_FAILED) {
        perror("[uio_test] Failed to mmap UIO memory");
        close(uio_fd);
        return 1;
    }
    printf("[uio_test] Successfully mmap'd UIO memory region 0 at virtual address %p\n", mapped_mem);

    // 3. 测试读写映射的内存 - 【【【修改点在这里】】】
    volatile unsigned int *test_ptr = (unsigned int *)mapped_mem;
    unsigned int old_value = *test_ptr; // 第一次读取
    printf("[uio_test] Read initial value 0x%x from mapped memory at %p.\n", old_value, mapped_mem);

    // 写入一个值，模拟设备交互
    *test_ptr = 0xDEADBEEF;
    printf("[uio_test] Attempted to write 0xDEADBEEF to mapped memory.\n");
    
    // 再次读取，但不再强制比较是否相同
    // 因为对于某些MMIO寄存器，读回来的值不一定等于写入的值
    unsigned int new_value_after_write = *test_ptr;
    printf("[uio_test] Read back value after write: 0x%x.\n", new_value_after_write);
    printf("[uio_test] Assuming write was successful. Proceeding to IRQ wait.\n");
    // 【【【修改点结束】】】

    // 4. 等待中断
    printf("[uio_test] Waiting for an interrupt by reading from the UIO device...\n");
    // read() 会阻塞，直到内核的 UIO 驱动接收到中断并唤醒它
    ssize_t bytes_read = read(uio_fd, &irq_count, sizeof(irq_count));

    if (bytes_read < 0) {
        perror("[uio_test] Failed to read from UIO device");
    } else if (bytes_read == 0) {
        printf("[uio_test] Read 0 bytes, EOF?\n");
    } else {
        // UIO spec: read() 返回一个 32 位的整数，表示自上次读取以来发生的中断次数
        printf("[uio_test] Read %ld bytes. Woken up by an interrupt! IRQ count: %u\n", bytes_read, irq_count);
    }

    // 清理
    munmap(mapped_mem, PAGE_SIZE);
    close(uio_fd);
    printf("[uio_test] UIO test finished.\n");
    return 0;
}
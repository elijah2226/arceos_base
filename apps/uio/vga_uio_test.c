// /starry/.arceos/apps/uio/vga_uio_test.c (最终动画修正版)

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <string.h>

#define VGA_MEM_SIZE 0x1000
#define VGA_WIDTH 80
#define VGA_HEIGHT 25

struct VgaChar {
    char character;
    unsigned char attribute;
};

// ---------------------------------------------------
// --- 视觉效果函数 (无改动) ---
// ---------------------------------------------------
void print_string(volatile struct VgaChar *buffer, int x, int y, const char *str, unsigned char attr) {
    int pos = y * VGA_WIDTH + x;
    char line_buffer[VGA_WIDTH + 1];
    memset(line_buffer, ' ', VGA_WIDTH);
    line_buffer[VGA_WIDTH] = '\0';
    strncpy(line_buffer, str, strlen(str));
    for (int i = 0; i < VGA_WIDTH - x; i++) {
        buffer[pos + i].character = line_buffer[i];
        buffer[pos + i].attribute = attr;
    }
}

void draw_rainbow_title(volatile struct VgaChar *buffer, const char *title, int frame) {
    int start_pos = (VGA_WIDTH - strlen(title)) / 2;
    for (int i = 0; i < strlen(title); i++) {
        unsigned char color = 1 + ((frame + i) % 14);
        buffer[start_pos + i].character = title[i];
        buffer[start_pos + i].attribute = color;
    }
}

void draw_marquee_border(volatile struct VgaChar *buffer, int frame) {
    unsigned char color = 1 + (frame % 14);
    char border_char = '*';
    for (int x = 0; x < VGA_WIDTH; x++) {
        buffer[x].character = border_char;
        buffer[x].attribute = color;
        buffer[(VGA_HEIGHT - 1) * VGA_WIDTH + x].character = border_char;
        buffer[(VGA_HEIGHT - 1) * VGA_WIDTH + x].attribute = color;
    }
    for (int y = 0; y < VGA_HEIGHT; y++) {
        buffer[y * VGA_WIDTH].character = border_char;
        buffer[y * VGA_WIDTH + (VGA_WIDTH - 1)].character = border_char;
        buffer[y * VGA_WIDTH + (VGA_WIDTH - 1)].attribute = color;
    }
}

// 【【【 新增：一个不依赖 syscall 的延时函数 】】】
void busy_sleep(long iterations) {
    // 使用 volatile 关键字防止编译器优化掉这个空循环
    for (volatile long i = 0; i < iterations; i++) {
        // Just burn CPU cycles
    }
}

// ---------------------------------------------------
// --- 主函数 ---
// ---------------------------------------------------
int main() {
    // ... (open 和 mmap 部分保持不变) ...
    int uio_fd;
    void *mapped_mem;
    printf("[vga_uio_test] 启动 UIO 视觉效果 Demo...\n");
    uio_fd = open("/dev/uio1", O_RDWR);
    mapped_mem = mmap(NULL, VGA_MEM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, uio_fd, 0);
    volatile struct VgaChar *vga_buffer = (volatile struct VgaChar *)mapped_mem;

    // 1. 初始画面
    for (int i = 0; i < VGA_WIDTH * VGA_HEIGHT; i++) {
        vga_buffer[i].character = ' ';
        vga_buffer[i].attribute = 0x07;
    }
    print_string(vga_buffer, 2, 2, "UIO Demo: Direct VGA Memory Control!", 0x0F);
    print_string(vga_buffer, 2, 4, "Press ENTER to start the first demo...", 0x0A);
    getchar();

    // 2. Demo 1: 动态彩虹标题 + 边框跑马灯
    print_string(vga_buffer, 2, 4, "Demo 1: Dynamic Rainbow Title & Marquee Border", 0x0E);
    for (int frame = 0; frame < 100; frame++) {
        draw_rainbow_title(vga_buffer, "--- UIO ROCKS! ---", frame);
        draw_marquee_border(vga_buffer, frame / 5);
        // 【【【 修改：使用我们自己的延时函数 】】】
        busy_sleep(10000000); // 这个数字需要根据你的 CPU 速度进行调整
    }
    print_string(vga_buffer, 2, 6, "Press ENTER to continue...", 0x0A);
    getchar();

    // 3. Demo 2: 实时计数器
    print_string(vga_buffer, 2, 6, "Demo 2: Real-time Counter (MMIO Write Speed)", 0x0E);
    for (int i = 0; i <= 500; i++) {
        char counter_str[20];
        sprintf(counter_str, "Counter: %d", i);
        print_string(vga_buffer, (VGA_WIDTH - 20) / 2, 12, "                    ", 0x4F);
        print_string(vga_buffer, (VGA_WIDTH - 20) / 2, 12, counter_str, 0x4F);
        // 【【【 修改：使用我们自己的延时函数 】】】
        busy_sleep(2500000); // 这个数字也需要调整
    }
    print_string(vga_buffer, 2, 8, "Press ENTER to exit...", 0x0A);
    getchar();

    // 清理
    munmap(mapped_mem, VGA_MEM_SIZE);
    close(uio_fd);
    printf("[vga_uio_test] UIO 视觉效果 Demo 结束。\n");
    return 0;
}
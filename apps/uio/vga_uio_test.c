// /starry/.arceos/apps/uio/vga_uio_test.c (最终英文视觉盛宴版)

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <string.h>

#define VGA_MEM_SIZE 0x1000
#define VGA_WIDTH 80
#define VGA_HEIGHT 25

// 定义一个结构体来方便地访问屏幕上的每个字符单元
struct VgaChar {
    char character;
    unsigned char attribute;
};

// ---------------------------------------------------
// --- 视觉效果函数 (无改动) ---
// ---------------------------------------------------

// 在指定位置用指定颜色写入字符串
void print_string(volatile struct VgaChar *buffer, int x, int y, const char *str, unsigned char attr) {
    int pos = y * VGA_WIDTH + x;
    // 增加一个清理旧内容的逻辑，避免长短不一的字符串留下痕迹
    char line_buffer[VGA_WIDTH + 1];
    memset(line_buffer, ' ', VGA_WIDTH);
    line_buffer[VGA_WIDTH] = '\0';
    strncpy(line_buffer, str, strlen(str));

    for (int i = 0; i < VGA_WIDTH - x; i++) {
        buffer[pos + i].character = line_buffer[i];
        buffer[pos + i].attribute = attr;
    }
}

// 绘制一个彩虹标题
void draw_rainbow_title(volatile struct VgaChar *buffer, const char *title, int frame) {
    int start_pos = (VGA_WIDTH - strlen(title)) / 2;
    // 颜色从 1 到 14 循环
    for (int i = 0; i < strlen(title); i++) {
        unsigned char color = 1 + ((frame + i) % 14);
        buffer[start_pos + i].character = title[i];
        buffer[start_pos + i].attribute = color;
    }
}

// 绘制边框跑马灯
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
        buffer[y * VGA_WIDTH].attribute = color;
        buffer[y * VGA_WIDTH + (VGA_WIDTH - 1)].character = border_char;
        buffer[y * VGA_WIDTH + (VGA_WIDTH - 1)].attribute = color;
    }
}


// ---------------------------------------------------
// --- 主函数 ---
// ---------------------------------------------------

int main() {
    int uio_fd;
    void *mapped_mem;

    printf("[vga_uio_test] 启动 UIO 视觉效果 Demo...\n");

    uio_fd = open("/dev/uio1", O_RDWR);
    if (uio_fd < 0) {
        perror("[vga_uio_test] 打开 /dev/uio1 失败");
        return 1;
    }

    mapped_mem = mmap(NULL, VGA_MEM_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, uio_fd, 0);
    if (mapped_mem == MAP_FAILED) {
        perror("[vga_uio_test] mmap VGA 显存失败");
        close(uio_fd);
        return 1;
    }

    volatile struct VgaChar *vga_buffer = (volatile struct VgaChar *)mapped_mem;

    // 1. 清空屏幕为黑色
    for (int i = 0; i < VGA_WIDTH * VGA_HEIGHT; i++) {
        vga_buffer[i].character = ' ';
        vga_buffer[i].attribute = 0x07; // 黑底白字
    }

    // 【【【 修改为英文 】】】
    print_string(vga_buffer, 2, 2, "UIO Demo: Direct VGA Memory Control!", 0x0F); // 黑底亮白
    print_string(vga_buffer, 2, 4, "Press ENTER to start the first demo...", 0x0A); // 黑底亮绿
    getchar();


    // 2. Demo 1: 动态彩虹标题 + 边框跑马灯
    // 【【【 修改为英文 】】】
    print_string(vga_buffer, 2, 4, "Demo 1: Dynamic Rainbow Title & Marquee Border", 0x0E); // 黑底黄字
    for (int frame = 0; frame < 100; frame++) {
        draw_rainbow_title(vga_buffer, "--- UIO ROCKS! ---", frame);
        draw_marquee_border(vga_buffer, frame / 5);
        usleep(50000); // 暂停 50 毫秒
    }
    // 【【【 修改为英文 】】】
    print_string(vga_buffer, 2, 6, "Press ENTER to continue...", 0x0A);
    getchar();


    // 3. Demo 2: 实时计数器
    // 【【【 修改为英文 】】】
    print_string(vga_buffer, 2, 6, "Demo 2: Real-time Counter (MMIO Write Speed)", 0x0E);
    for (int i = 0; i <= 500; i++) {
        char counter_str[20];
        sprintf(counter_str, "Counter: %d", i);
        // 先用一个背景色块清空区域，避免数字长度变化留下残影
        print_string(vga_buffer, (VGA_WIDTH - 20) / 2, 12, "                    ", 0x4F); // 红底亮白
        print_string(vga_buffer, (VGA_WIDTH - 20) / 2, 12, counter_str, 0x4F); // 红底亮白
        usleep(10000); // 暂停 10 毫秒
    }
    // 【【【 修改为英文 】】】
    print_string(vga_buffer, 2, 8, "Press ENTER to exit...", 0x0A);
    getchar();


    // 清理
    munmap(mapped_mem, VGA_MEM_SIZE);
    close(uio_fd);
    printf("[vga_uio_test] UIO 视觉效果 Demo 结束。\n");
    return 0;
}
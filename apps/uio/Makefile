# apps/uio_test/Makefile

# 假设你的交叉编译器前缀是 x86_64-linux-gnu-
# 如果是 riscv64, 就换成 riscv64-linux-gnu-
CROSS_COMPILE ?= x86_64-linux-gnu-
CC := $(CROSS_COMPILE)gcc
STRIP := $(CROSS_COMPILE)strip

# CFLAGS: -static 表示静态链接，-O2 是优化等级
CFLAGS := -static -g -Wall
LDFLAGS := -static

TARGET := uio_test
SRCS := uio_test.c

# -------------------------------------------------------------

OBJS := $(SRCS:.c=.o)
BUILD_DIR := build

.PHONY: all build clean

all: build

build: $(BUILD_DIR)/$(TARGET)

$(BUILD_DIR)/$(TARGET): $(OBJS)
	@mkdir -p $(BUILD_DIR)
	$(CC) $(LDFLAGS) -o $@ $^
# @$(STRIP) $@

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

clean:
	rm -rf $(BUILD_DIR) *.o
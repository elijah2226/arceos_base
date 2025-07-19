# apps/uio/Makefile

# 交叉编译器配置 (保持不变)
CROSS_COMPILE ?= x86_64-linux-gnu-
CC := $(CROSS_COMPILE)gcc
STRIP := $(CROSS_COMPILE)strip

# 编译和链接标志 (保持不变)
CFLAGS := -static -g -Wall
LDFLAGS := -static

TARGETS := uio_test hpet_uio_test vga_uio_test

BUILD_DIR := build

# 根据 TARGETS 列表和 BUILD_DIR 自动生成所有最终程序的完整路径。
# 例如，这会生成 "build/uio_test build/hpet_uio_test"
TARGET_BINS := $(addprefix $(BUILD_DIR)/, $(TARGETS))

# -------------------------------------------------------------

# .PHONY 定义了伪目标，它们不代表真实文件
.PHONY: all build clean

# 默认目标是 build
all: build

# build 目标现在依赖于 TARGET_BINS 列表中的所有程序。
# make 会自动为列表中的每个程序寻找构建规则。
build: $(TARGET_BINS)

# 这是一条强大的模式规则，它告诉 make 如何从任何一个 .c 文件创建对应的可执行文件。
# $@ 代表目标 (例如 build/hpet_uio_test)
# $< 代表第一个依赖 (例如 hpet_uio_test.c)
$(BUILD_DIR)/%: %.c
	@mkdir -p $(BUILD_DIR)
	@echo "  CC    $@ (from $<)"
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<
#	@$(STRIP) $@

# 清理规则 (保持不变，它会删除整个 build 目录)
clean:
	rm -rf $(BUILD_DIR) *.o
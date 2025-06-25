# Build testcases for rust and c programs

# 1. 架构与目标库定义
ARCH ?= x86_64
# Whether cross-compiling
# musl 是一个轻量级的、专注于静态链接的标准 C 库实现。
TARGET ?= musl

PREFIX := $(ARCH)-linux-$(TARGET)

# 2. 构造交叉编译器名称
# Build target for c programs
CC := $(PREFIX)-gcc

# CC 被设置为 x86_64-linux-musl-gcc (在默认情况下)。
# 证明了它不使用开发者主机上的默认 gcc，而是调用一个特定的交叉编译工具链。
# 这个工具链被设计用来生成能在 starry-next (作为目标系统) 上运行，并与 musl C 库链接的可执行文件。

# 3. 设置编译标志
CFLAGS := 
ifeq ($(TARGET), musl)
# -static 标志告诉编译器将所有需要的库函数（这里是 musl 的函数）的代码直接编译进最终的可执行文件中。
  CFLAGS += -static
endif

all: build

build: build_dir build_c

build_dir:
	@mkdir -p build
	@mkdir -p build/$(ARCH)

# 4. 编译循环
# 构建过程非常直接：找到源文件 -> 调用交叉编译器 -> 生成可执行文件。
# 它没有链接任何像 nimbos 那样的本地库 (libulib.a)。
build_c:
  # No build for loongarch64
	for app in $(wildcard c/*/*.c); do \
		echo "Building $${app%.c}"; \
		app_name=$$(basename $$(dirname $${app})); \
		$(CC) -o build/$(ARCH)/$${app_name}_c $${app} $(CFLAGS); \
	done

clean:
	@rm -rf build

.PHONY: all build_dir build_c build_rust clean
# ==============================================================================
# ArceOS Unikernel Makefile (最终版, 目标驱动的双模设计)
# ==============================================================================

# ------------------------------------------------------------------------------
# Part 1: 高层配置与默认设置
# ------------------------------------------------------------------------------
# 构建场景 (BUILD_SCENARIO) 现在由 make 目标 (run, test, debug) 控制
# 如果是直接调用 'make build' 等底层命令，则默认为 'normal'
BUILD_SCENARIO ?= normal

# 默认构建的应用。可以被覆盖。
APP ?= arceos-main

# 默认的硬件/功能设置。可以在命令行中覆盖。
ARCH ?= x86_64
LOG  ?= info
SMP  ?= 1
BUS  ?= pci
MEM  ?= 128M
NET  ?= y

# 根据 BUILD_SCENARIO 决定 MODE (release/debug 编译模式)
ifeq ($(BUILD_SCENARIO), test)
    MODE := debug
else ifeq ($(BUILD_SCENARIO), debug)
    MODE := debug
else
    MODE := release
endif

# --- 测试模式专属设置 ---
# 这些设置仅在 BUILD_SCENARIO=test 时生效
ifeq ($(BUILD_SCENARIO), test)
    AX_TESTCASE ?= nimbos # 默认的测试套件
    AX_TESTCASES_LIST := $(shell cat ./apps/$(AX_TESTCASE)/testcase_list 2>/dev/null | tr '\n' ',')
    export AX_TESTCASES_LIST
endif

# ------------------------------------------------------------------------------
# Part 2: 基于场景的特性与构建配置
# ------------------------------------------------------------------------------
NO_AXSTD := y
EXTRA_CONFIG ?= $(PWD)/configs/Monolithic/$(ARCH).toml

# 根据选择的场景决定内核特性
# 使用一个临时变量来构建特性列表
_FEATURES :=

_FEATURES += linux_compat

_FEATURES += log-level-$(LOG)
ifeq ($(ARCH), aarch64)
    _FEATURES += fp_simd
endif

# 将自动选择的特性与用户额外指定的特性合并，形成最终的 FEATURES 变量
override FEATURES := $(_FEATURES) $(FEATURES)

# ------------------------------------------------------------------------------
# Part 3: ArceOS 核心构建系统结构
# ------------------------------------------------------------------------------
# 这部分是构建引擎的核心，基本保持不变。
PLATFORM ?=
V ?=
TARGET_DIR ?= $(PWD)/target
APP_FEATURES ?=
UIMAGE ?= n
ifeq ($(strip $(IP)),)
  IP = 10.0.2.15
endif
ifeq ($(strip $(GW)),)
  GW = 10.0.2.2
endif
DISK_IMG ?= $(if $(filter test,$(BUILD_SCENARIO)),$(AX_TESTCASE)_disk.img,disk.img)
NET_DEV ?= user

ifneq ($(wildcard $(APP)/Cargo.toml),)
  APP_TYPE := rust
else
  APP_TYPE := c
endif

include scripts/make/features.mk
include scripts/make/platform.mk

ifeq ($(ARCH), x86_64)
  TARGET := x86_64-unknown-none
else ifeq ($(ARCH), aarch64)
  TARGET := aarch64-unknown-none
else ifeq ($(ARCH), riscv64)
  TARGET := riscv64gc-unknown-none-elf
else
  $(error "不支持的架构: $(ARCH)")
endif
export AX_ARCH=$(ARCH)
export AX_PLATFORM=$(PLAT_NAME)
export AX_MODE=$(MODE)
export AX_LOG=$(LOG)
export AX_TARGET=$(TARGET)
OUT_CONFIG := $(PWD)/.axconfig.toml
export AX_CONFIG_PATH=$(OUT_CONFIG)
export AX_IP=$(IP)
export AX_GW=$(GW)

OBJDUMP ?= rust-objdump -d --print-imm-hex --x86-asm-syntax=intel
OBJCOPY ?= rust-objcopy --binary-architecture=$(ARCH)
GDB ?= gdb-multiarch

OUT_DIR ?= $(APP)
APP_NAME := $(shell basename $(APP))
LD_SCRIPT := $(TARGET_DIR)/$(TARGET)/$(MODE)/linker_$(PLAT_NAME).lds
OUT_ELF := $(OUT_DIR)/$(APP_NAME)_$(PLAT_NAME).elf
OUT_BIN := $(patsubst %.elf,%.bin,$(OUT_ELF))
OUT_UIMG := $(patsubst %.elf,%.uimg,$(OUT_ELF))
ifeq ($(UIMAGE), y)
  FINAL_IMG := $(OUT_UIMG)
else
  FINAL_IMG := $(OUT_BIN)
endif

include scripts/make/utils.mk
include scripts/make/config.mk
include scripts/make/build.mk
include scripts/make/qemu.mk

# ------------------------------------------------------------------------------
# Part 4: 面向用户的高层目标 (升级优化版)
# ------------------------------------------------------------------------------
.PHONY: all build run test debug justrun disasm oldconfig defconfig clean user_apps help

all: build

# 'build' 是一个通用目标，它会尊重命令行传入的设置。
# 例如, 'make build BUILD_SCENARIO=test' 会执行一次测试构建。
build: user_apps $(OUT_DIR) $(FINAL_IMG)

# 目标：用于常规执行 (Release 模式)
run:
	@+$(MAKE) build BUILD_SCENARIO=normal $(filter-out $@,$(MAKECMDGOALS))
	@+$(MAKE) justrun BUILD_SCENARIO=normal $(filter-out $@,$(MAKECMDGOALS))

# 目标：用于运行集成测试 (Test 模式)
test:
	# 方案一: 直接调用外部脚本
	@echo "======> 正在通过脚本运行集成测试场景... ======"
	@echo "测试套件: $(or $(AX_TESTCASE),nimbos)"
	# 假设脚本会处理所有事情，包括构建。
	# 如果需要，可以向脚本传递 Makefile 变量。
	@bash ./scripts/test/app_test.sh $(AX_TESTCASE)
	
	# 方案二: 使用 Makefile 自己的构建系统 (集成度更高)
	# 要使用此方案，请注释掉方案一并取消下面的注释。
	# @echo "======> 正在通过 Makefile 运行集成测试场景... ======"
	# @+$(MAKE) build BUILD_SCENARIO=test $(filter-out $@,$(MAKECMDGOALS))
	# @+$(MAKE) justrun BUILD_SCENARIO=test $(filter-out $@,$(MAKECMDGOALS))

# 目标：用于调试会话 (Debug 模式)
debug:
	@echo "======> 正在进入 DEBUG 场景..."
	@+$(MAKE) build BUILD_SCENARIO=debug $(filter-out $@,$(MAKECMDGOALS))
	$(call run_qemu_debug)

# 内部目标，仅运行 QEMU 而不重新构建。
justrun:
	$(call run_qemu)

disasm: build
	$(OBJDUMP) $(OUT_ELF) | less

# 这个目标的逻辑现在由 BUILD_SCENARIO 驱动。
user_apps:
ifeq ($(BUILD_SCENARIO), test)
	@echo "====== 正在为 TEST 场景构建用户应用 (测试用例: $(AX_TESTCASE)) ======"
	@$(MAKE) -C ./apps/$(AX_TESTCASE) ARCH=$(ARCH) build
	@echo "====== 正在为测试用例创建磁盘镜像... ======"
	# 注意这里的脚本调用，它会格式化磁盘
	@./scripts/test/build_img.sh -a $(ARCH) -file ./apps/$(AX_TESTCASE)/build/$(ARCH) -s 20
	@echo "====== 正在将 disk.img 重命名为 $(DISK_IMG) ======"
	@mv disk.img $(DISK_IMG)
else
	@echo "====== 正在为 NORMAL/DEBUG 场景构建用户应用 ======"
	@if [ ! -f "$(DISK_IMG)" ]; then \
		echo "创建空磁盘镜像: $(DISK_IMG)"; \
		qemu-img create -f raw $(DISK_IMG) 20M; \
		echo "格式化磁盘镜像为 FAT32: $(DISK_IMG)"; \
		mkfs.fat $(DISK_IMG); \
	fi
endif

defconfig: _axconfig-gen
	$(call defconfig)

oldconfig: _axconfig-gen
	$(call oldconfig)

# clean:
# 	@echo "正在清理内核产物..."
# 	@rm -rf $(APP)/*.bin $(APP)/*.elf $(OUT_CONFIG) *.img
# 	@cargo clean
# 	@echo "正在清理所有用户应用套件..."
# 	@for dir in $(shell find apps/* -maxdepth 0 -type d); do \
# 		if [ -d "$$dir" ]; then $(MAKE) -C $$dir clean; fi \
# 	done

clean:
	# ... (清理内核产物)
	@echo "Cleaning all user app suites..."
	@for mkfile in $(shell find apps/*/Makefile); do \
		$(MAKE) -C $$(dirname $$mkfile) clean; \
	done

help:
	@echo "ArceOS Unikernel Build System"
	@echo ""
	@echo "Usage: make [TARGET] [VAR=VALUE]..."
	@echo ""
	@echo "Main Targets:"
	@printf "  %-20s %s\n" "run" "Build and run the system in normal (release) mode."
	@printf "  %-20s %s\n" "test" "Run integration tests (usually calls an external script)."
	@printf "  %-20s %s\n" "debug" "Build in debug mode and start QEMU for GDB debugging."
	@printf "  %-20s %s\n" "build" "Build the system without running. Use VARs to control."
	@printf "  %-20s %s\n" "clean" "Clean all build artifacts from kernel and apps."
	@printf "  %-20s %s\n" "help" "Show this help message."
	@echo ""
	@echo "Common Variables:"
	@printf "  %-20s %s\n" "ARCH" "Target architecture (e.g., x86_64, aarch64). Default: $(ARCH)"
	@printf "  %-20s %s\n" "NET" "Enable network (y/n). Default: $(NET)"
	@printf "  %-20s %s\n" "AX_TESTCASE" "Specify test suite for 'make test'. Default: $(or $(AX_TESTCASE),nimbos)"
	@printf "  %-20s %s\n" "V" "Verbose build (V=1 or V=2)."

# 并且可以设置一个默认目标
.DEFAULT_GOAL := help
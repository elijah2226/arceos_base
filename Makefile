# ==============================================================================
# ArceOS Unikernel Makefile
# ==============================================================================

.DEFAULT_GOAL := help
.PHONY: all build run test debug justrun disasm oldconfig defconfig clean user_apps help

# ------------------------------------------------------------------------------
# Part 1: 用户配置与派生设置
# ------------------------------------------------------------------------------
# --- 用户可覆盖的默认值 (User-overridable Defaults) ---
APP  		?= arceos-main
ARCH 		?= x86_64
LOG  		?= info
SMP  		?= 1
BUS  		?= pci
MEM  		?= 128M
NET  		?= y
IP 			?= 10.0.2.15
GW 			?= 10.0.2.2
PLATFORM 	?=
V 			?=

# --- 场景化配置 (Scenario-driven Configuration) ---
BUILD_SCENARIO ?= normal
MODE := release
ifneq (,$(filter $(BUILD_SCENARIO),test debug))
    MODE := debug
endif
# --- 测试模式专属设置 ---
ifeq ($(BUILD_SCENARIO), test)
    AX_TESTCASE ?= nimbos
    export AX_TESTCASES_LIST := $(shell cat ./apps/$(AX_TESTCASE)/testcase_list 2>/dev/null | tr '\n' ',')
endif

# ------------------------------------------------------------------------------
# Part 2: 构建特性与路径配置 (Build Features & Paths)
# ------------------------------------------------------------------------------
# --- 核心路径定义 (Core Path Definitions) ---
TARGET_DIR   	?= $(CURDIR)/target
EXTRA_CONFIG 	?= $(CURDIR)/configs/Monolithic/$(ARCH).toml
OUT_CONFIG 		:= $(CURDIR)/.axconfig.toml
OUT_DIR 		?= $(APP)

# --- 特性组装 (Feature Assembly) ---
include scripts/make/features.mk
NO_AXSTD 		:= y
APP_FEATURES 	?=
AUTO_FEATURES := linux_compat log-level-$(LOG)
ifeq ($(ARCH), aarch64)
    AUTO_FEATURES += fp_simd
endif
override FEATURES := $(AUTO_FEATURES) $(FEATURES)

# --- 目标架构与产物定义 (Target & Artifacts Setup) ---
include scripts/make/platform.mk
TARGET_MAP.x86_64   := x86_64-unknown-none
TARGET_MAP.aarch64  := aarch64-unknown-none
TARGET_MAP.riscv64  := riscv64gc-unknown-none-elf
TARGET := $(TARGET_MAP.$(ARCH))
ifeq ($(TARGET),)
  $(error "Unsupported architecture: $(ARCH). Check TARGET_MAP.")
endif

APP_NAME 	:= $(shell basename $(APP))
LD_SCRIPT 	:= $(TARGET_DIR)/$(TARGET)/$(MODE)/linker_$(PLAT_NAME).lds
OUT_ELF 	:= $(OUT_DIR)/$(APP_NAME)_$(PLAT_NAME).elf
OUT_BIN 	:= $(patsubst %.elf,%.bin,$(OUT_ELF))
OUT_UIMG 	:= $(patsubst %.elf,%.uimg,$(OUT_ELF))
DISK_IMG ?= $(if $(filter test,$(BUILD_SCENARIO)),$(AX_TESTCASE)_disk.img,disk.img)
UIMAGE ?= n
ifeq ($(UIMAGE), y)
  FINAL_IMG := $(OUT_UIMG)
else
  FINAL_IMG := $(OUT_BIN)
endif
ifneq ($(wildcard $(APP)/Cargo.toml),)
  APP_TYPE := rust
else
  APP_TYPE := c
endif

# ------------------------------------------------------------------------------
# Part 3: 构建引擎与环境
# ------------------------------------------------------------------------------
# --- 工具链定义 (Toolchain) ---
NET_DEV ?= user
OBJDUMP ?= rust-objdump -d --print-imm-hex --x86-asm-syntax=intel
OBJCOPY ?= rust-objcopy --binary-architecture=$(ARCH)
GDB ?= gdb-multiarch

# --- 环境变量导出 (Exported Environment) ---
export AX_ARCH			=$(ARCH)
export AX_PLATFORM		=$(PLAT_NAME)
export AX_MODE			=$(MODE)
export AX_LOG			=$(LOG)
export AX_TARGET		=$(TARGET)
export AX_CONFIG_PATH	=$(OUT_CONFIG)
export AX_IP			=$(IP)
export AX_GW			=$(GW)

# --- 包含模块化的 Makefile 片段 ---
include scripts/make/utils.mk
include scripts/make/config.mk
include scripts/make/build.mk
include scripts/make/qemu.mk

# ------------------------------------------------------------------------------
# Part 4: 用户交互目标
# ------------------------------------------------------------------------------

all: build

build: user_apps $(FINAL_IMG)

run:
	@echo "======> SCENARIO: Normal Run (Release Mode)"
	@+$(MAKE) build BUILD_SCENARIO=normal $(filter-out $@,$(MAKECMDGOALS))
	@+$(MAKE) justrun BUILD_SCENARIO=normal $(filter-out $@,$(MAKECMDGOALS))

test:
	@echo "======> SCENARIO: Integration Test"
	@echo "Test suite: $(or $(AX_TESTCASE),nimbos)"
	@bash ./scripts/test/app_test.sh $(or $(AX_TESTCASE),nimbos)

debug:
	@echo "======> SCENARIO: Debug Session (Debug Mode)"
	@+$(MAKE) build BUILD_SCENARIO=debug $(filter-out $@,$(MAKECMDGOALS))
	$(call run_qemu_debug)

justrun:
	$(call run_qemu)

disasm: build
	$(OBJDUMP) $(OUT_ELF) | less

user_apps:
ifeq ($(BUILD_SCENARIO), test)
	@echo "====== Building user apps for TEST scenario (Testcase: $(AX_TESTCASE))"
	@$(MAKE) -C ./apps/$(AX_TESTCASE) ARCH=$(ARCH) build
	@echo "====== Creating disk image for testcase..."
	@./scripts/test/build_img.sh -a $(ARCH) -file ./apps/$(AX_TESTCASE)/build/$(ARCH) -s 20
	@echo "====== Renaming disk.img to $(DISK_IMG)"
	@mv disk.img $(DISK_IMG)
else
	@echo "====== Preparing user apps for NORMAL/DEBUG scenario"
	@if [ ! -f "$(DISK_IMG)" ]; then \
		echo "Disk image '$(DISK_IMG)' not found. Creating a new one."; \
		qemu-img create -f raw $(DISK_IMG) 20M; \
		echo "Formatting '$(DISK_IMG)' as FAT32."; \
		mkfs.fat $(DISK_IMG); \
	fi
endif

defconfig: _axconfig-gen
	$(call defconfig)

oldconfig: _axconfig-gen
	$(call oldconfig)

APP_MAKEFILES := $(wildcard apps/*/Makefile)
APP_CLEAN_DIRS := $(patsubst %/Makefile,%,$(APP_MAKEFILES))
clean:
	@echo "Cleaning kernel artifacts..."
	@rm -rf $(APP)/*.bin $(APP)/*.elf $(APP)/*.uimg $(OUT_CONFIG) *.img
	@cargo clean
	@echo "Cleaning all user app suites..."
	@if [ -n "$(APP_CLEAN_DIRS)" ]; then \
		for dir in $(APP_CLEAN_DIRS); do \
			echo "===> Cleaning $$dir"; \
			$(MAKE) -C $$dir clean; \
		done \
	fi

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
	@printf "  %-20s %s\n" "AX_TESTCASE" "Specify test suite for 'make test'. Default: $(AX_TESTCASE)"
	@printf "  %-20s %s\n" "V" "Verbose build (V=1 or V=2)."
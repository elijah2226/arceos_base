# ==============================================================================
# ArceOS Unikernel Makefile (Final, Corrected Dual-Mode)
# ==============================================================================

# ------------------------------------------------------------------------------
# Part 1: High-Level Configuration & Mode Selection (高层配置与模式选择)
# ------------------------------------------------------------------------------
ARCH ?= x86_64
LOG  ?= info
SMP  ?= 1
BUS  ?= pci
MEM  ?= 128M
# BUILD_SCENARIO can be 'normal' (default) or 'test'
BUILD_SCENARIO ?= normal

# MODE must be 'release' or 'debug' for cargo
ifeq ($(BUILD_SCENARIO), test)
    MODE ?= debug
else
    MODE ?= release
endif

# --- Test-Mode Specific Settings --- (测试模式专属设置)
ifeq ($(BUILD_SCENARIO), test)
    AX_TESTCASE ?= nimbos
    AX_TESTCASES_LIST := $(shell cat ./apps/$(AX_TESTCASE)/testcase_list 2>/dev/null | tr '\n' ',')
    export AX_TESTCASES_LIST
endif

# ------------------------------------------------------------------------------
# Part 2: Feature & Build Configuration based on SCENARIO
# ------------------------------------------------------------------------------
NO_AXSTD := y
APP  ?= arceos-main
EXTRA_CONFIG ?= $(PWD)/configs/Monolithic/$(ARCH).toml

# Determine kernel features based on the selected SCENARIO
ifeq ($(BUILD_SCENARIO), test)
    override FEATURES += linux_compat
else
    override FEATURES += linux_normal_mode
endif
override FEATURES += log-level-$(LOG)
ifeq ($(ARCH), aarch64)
    override FEATURES += fp_simd
endif

# ------------------------------------------------------------------------------
# Part 3: Replicate the Original ArceOS Build System Structure
# ------------------------------------------------------------------------------
# This section is now 100% aligned with the original, working Makefile.
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
  $(error "Unsupported ARCH: $(ARCH)")
endif
export AX_ARCH=$(ARCH)
export AX_PLATFORM=$(PLAT_NAME)
export AX_MODE=$(MODE) # <-- This now correctly passes 'release' or 'debug'
export AX_LOG=$(LOG)
export AX_TARGET=$(TARGET)
OUT_CONFIG := $(PWD)/.axconfig.toml
export AX_CONFIG_PATH=$(OUT_CONFIG)
export AX_IP=$(IP)
# 将 Makefile 内部的 IP 变量导出为 AX_IP
export AX_GW=$(GW)
# 将 Makefile 内部的 GW 变量导出为 AX_GW

# --- START DEBUGGING LOGS ---
.PHONY: debug-vars
debug-vars:
	@echo "--- Makefile Debug Info ---"
	@echo "Current IP (Makefile var): $(IP)"
	@echo "Current GW (Makefile var): $(GW)"
	@echo "AX_IP (exported env): $(AX_IP)"
	@echo "AX_GW (exported env): $(AX_GW)"
	@echo "BUILD_SCENARIO: $(BUILD_SCENARIO)"
	@echo "--- End Makefile Debug Info ---"
# --- END DEBUGGING LOGS ---

OBJDUMP ?= rust-objdump -d --print-imm-hex --x86-asm-syntax=intel
OBJCOPY ?= rust-objcopy --binary-architecture=$(ARCH)
GDB ?= gdb-multiarch

OUT_DIR ?= $(APP)
APP_NAME := $(shell basename $(APP))
LD_SCRIPT := $(TARGET_DIR)/$(TARGET)/$(MODE)/linker_$(PLAT_NAME).lds # <-- Now MODE is correct
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
# Part 4: High-Level User-Facing Targets (Upgraded)
# ------------------------------------------------------------------------------
.PHONY: all build run test justrun debug disasm oldconfig defconfig clean user_apps

all: build
build: user_apps $(OUT_DIR) $(FINAL_IMG)

run: build
	$(call run_qemu)

justrun: build
	$(call run_qemu)

# The 'test' target is a shortcut for running in test scenario
test:
	@echo "======> Entering TEST scenario..."


justrun: run
debug: build
	# ... (debug command)

user_apps:
ifeq ($(BUILD_SCENARIO), test)
	@echo "====== Building User Apps for TEST scenario (testcase: $(AX_TESTCASE)) ======"
	@$(MAKE) -C ./apps/$(AX_TESTCASE) ARCH=$(ARCH) build
	@echo "====== Creating disk image for testcases... ======"
	@./scripts/test/build_img.sh -a $(ARCH) -file ./apps/$(AX_TESTCASE)/build/$(ARCH) -s 20
	@echo "====== Renaming disk.img to $(DISK_IMG) ======"
	@mv disk.img $(DISK_IMG)
else
	@echo "====== Building User Apps for NORMAL scenario (not implemented yet) ======"
	@if [ ! -f "$(DISK_IMG)" ]; then \
		echo "Creating empty disk image: $(DISK_IMG)"; \
		qemu-img create -f raw $(DISK_IMG) 20M; \
	fi
endif

disasm: build
	$(OBJDUMP) $(OUT_ELF) | less
defconfig: _axconfig-gen
	$(call defconfig)
oldconfig: _axconfig-gen
	$(call oldconfig)

clean:
	@echo "Cleaning kernel artifacts..."
	@rm -rf $(APP)/*.bin $(APP)/*.elf $(OUT_CONFIG) *.img
	@cargo clean
	@echo "Cleaning all user app suites..."
	@for dir in $(shell find apps/* -maxdepth 0 -type d); do \
		if [ -d "$$dir" ]; then $(MAKE) -C $$dir clean; fi \
	done
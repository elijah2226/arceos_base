# /starry/.arceos/scripts/make/build.mk (最终优化版)

include scripts/make/cargo.mk

# ==============================================================================
# Part 1: 构建包与产物路径定义
# ==============================================================================

ifeq ($(APP_TYPE), c)
  BUILD_PACKAGE := axlibc
else
  BUILD_PACKAGE := $(APP_NAME)
endif

CARGO_ELF_SRC := $(TARGET_DIR)/$(TARGET)/$(MODE)/$(BUILD_PACKAGE)

# ==============================================================================
# Part 2: 构建标志位准备 (Flags Preparation)
# ==============================================================================

# 只有在执行真正的构建目标时，才添加链接参数
ifneq ($(filter $(BUILD_SCENARIO), normal debug test),)
  ifeq ($(APP_TYPE), rust)
    RUSTFLAGS += $(RUSTFLAGS_LINK_ARGS)
  endif
endif

# 为 doc 目标添加专属 RUSTDOCFLAGS
ifneq ($(filter doc doc_check_missing,$(MAKECMDGOALS)),)
  export RUSTDOCFLAGS
endif

export RUSTFLAGS

# ==============================================================================
# Part 3: 构建规则 (Build Rules)
# ==============================================================================

# --- 核心 Cargo 构建步骤 ---
.PHONY: _cargo_build
_cargo_build: oldconfig
	@echo
	@echo "--- Build Info (Scenario: $(BUILD_SCENARIO)) ---"
	@echo "  App Name:         $(APP_NAME) (Type: $(APP_TYPE))"
	@echo "  Package to build: $(BUILD_PACKAGE)"
	@echo "  Build Mode:       $(MODE)"
	@echo "  Final Features:   $(FINAL_FEATURES)"
	@echo "------------------------------------------------"
	@printf "    $(GREEN_C)Building$(END_C) Cargo package: $(BUILD_PACKAGE)\n"
	$(call cargo_build,$(BUILD_PACKAGE),$(FINAL_FEATURES))

# --- 产物生成规则 ---
$(OUT_ELF): _cargo_build
	@printf "    $(GREEN_C)Copying$(END_C)  ELF: $(CARGO_ELF_SRC) -> $@\n"
	@mkdir -p $(dir $@)
	@cp $(CARGO_ELF_SRC) $@

$(OUT_BIN): $(OUT_ELF)
	@printf "    $(GREEN_C)Creating$(END_C) BIN: $(notdir $@)\n"
	$(call run_cmd,$(OBJCOPY),$< --strip-all -O binary $@)

# --- U-Boot 镜像规则 (使用变量映射优化) ---
UIMG_ARCH.aarch64 := arm64
UIMG_ARCH.riscv64 := riscv
uimg_arch := $(or $(UIMG_ARCH.$(ARCH)),$(ARCH))

$(OUT_UIMG): $(OUT_BIN)
	@printf "    $(GREEN_C)Creating$(END_C) U-Boot Image: $(notdir $@)\n"
	$(call run_cmd,mkimage,\
		-A $(uimg_arch) -O linux -T kernel -C none \
		-a $(subst _,,$(shell axconfig-gen "$(OUT_CONFIG)" -r plat.kernel-base-paddr)) \
		-d $< $@)
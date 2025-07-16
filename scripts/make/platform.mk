# ==============================================================================
# Architecture and Platform Resolver (Refactored for Clarity)
#
# This script determines the target platform (`PLAT_NAME`) and its configuration
# file (`PLAT_CONFIG`) based on the `ARCH` and `PLATFORM` variables.
#
# Logic Flow:
# 1. If `PLATFORM` is specified, it takes precedence and determines `ARCH`.
# 2. If `PLATFORM` is not specified, a default platform is chosen based on `ARCH`.
#
# Inputs:
#   - ARCH: The target architecture (e.g., x86_64), may be overridden.
#   - PLATFORM: (Optional) The target platform name or path to a config file.
#
# Outputs:
#   - PLAT_NAME: The final platform identifier (e.g., x86_64-qemu-q35).
#   - PLAT_CONFIG: The path to the platform's TOML configuration file.
#   - ARCH: The final architecture, possibly updated based on PLATFORM.
# ==============================================================================

# ------------------------------------------------------------------------------
# Part 1: 处理 PLATFORM 变量 (当它被指定时)
# ------------------------------------------------------------------------------
# 只有在 PLATFORM 变量不为空时，才执行此块内的逻辑。
ifneq ($(PLATFORM),)
    # --- 探测所有内置平台 ---
    BUILTIN_PLATFORMS := $(patsubst configs/platforms/%.toml,%,$(wildcard configs/platforms/*.toml))

    # --- 判断 PLATFORM 的类型 ---
    ifeq ($(filter $(PLATFORM),$(BUILTIN_PLATFORMS)),$(PLATFORM))
        # --- 情况 A: PLATFORM 是一个已知的内置平台 ---
        _determined_arch := $(word 1,$(subst -, ,$(PLATFORM)))
        PLAT_NAME        := $(PLATFORM)
        PLAT_CONFIG      := configs/platforms/$(PLAT_NAME).toml
    else ifneq ($(wildcard $(PLATFORM)),)
        # --- 情况 B: PLATFORM 是一个存在的自定义 TOML 文件路径 ---
        # 调用外部工具读取文件内容，来确定架构和平台名
        _determined_arch := $(patsubst "%",%,$(shell axconfig-gen $(PLATFORM) -r arch))
        PLAT_NAME        := $(patsubst "%",%,$(shell axconfig-gen $(PLATFORM) -r platform))
        PLAT_CONFIG      := $(PLATFORM)
    else
        # --- 情况 C: PLATFORM 无效 ---
        $(error "PLATFORM='$(PLATFORM)' is not a built-in platform ($(BUILTIN_PLATFORMS)) nor a valid file path.")
    endif

    # --- 校验与覆盖 ARCH ---
    # `origin ARCH` 检查 ARCH 变量是否来自命令行
    ifeq ($(origin ARCH), command line)
        ifneq ($(ARCH),$(_determined_arch))
            $(error "ARCH='$(ARCH)' on command line is incompatible with PLATFORM='$(PLATFORM)', which requires ARCH='$(_determined_arch)'.")
        endif
    endif
    ARCH := $(_determined_arch)
endif

# ------------------------------------------------------------------------------
# Part 2: 处理 ARCH 变量 (当 PLATFORM 未指定或处理完毕后)
# ------------------------------------------------------------------------------
# --- 定义默认平台查找表 ---
# 使用变量映射替代 if-else 链，清晰且易于扩展。
DEFAULT_PLAT_MAP.x86_64      := x86_64-qemu-q35
DEFAULT_PLAT_MAP.aarch64     := aarch64-qemu-virt
DEFAULT_PLAT_MAP.riscv64     := riscv64-qemu-virt
DEFAULT_PLAT_MAP.loongarch64 := loongarch64-qemu-virt

# --- 为未指定 PLATFORM 的情况设置默认值 ---
ifneq ($(PLAT_NAME),)
    # PLAT_NAME 已由 Part 1 设置，无需操作。
else
    # PLAT_NAME 为空，说明需要根据 ARCH 来设置默认平台。
    PLAT_NAME := $(DEFAULT_PLAT_MAP.$(ARCH))
    ifeq ($(PLAT_NAME),)
        $(error "Unsupported ARCH='$(ARCH)'. Must be one of: $(patsubst %,%,$(.VARIABLES)))")
    endif
    PLAT_CONFIG := configs/platforms/$(PLAT_NAME).toml
endif

# ------------------------------------------------------------------------------
# Part 3: 清理临时变量 (可选，但保持命名空间干净是好习惯)
# ------------------------------------------------------------------------------
_determined_arch  :=
BUILTIN_PLATFORMS :=
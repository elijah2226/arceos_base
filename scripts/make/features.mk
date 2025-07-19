# ==============================================================================
# Feature Resolver for ArceOS (Smart Partitioner Version)
#
# This script collects features, partitions them into "kernel-level" and
# "top-level" groups, applies the correct prefixes, and assembles the final
# feature string for Cargo.
#
# Inputs:
#   - FEATURES: Space/comma-separated list of features from the command line.
#   - LOG, BUS, SMP, etc.: High-level config variables.
#
# Outputs:
#   - FINAL_FEATURES: A clean, comma-separated string of all enabled features
#                     ready to be passed to `cargo build`.
# ==============================================================================

# ------------------------------------------------------------------------------
# Part 1: 配置与清单定义
# ------------------------------------------------------------------------------
# --- 内核特性清单 ---
_KERNEL_FEATURE_LIST := \
    smp fp_simd irq alloc alloc-tlsf alloc-slab alloc-buddy page-alloc-64g \
    page-alloc-4g paging dma tls multitask sched_fifo sched_rr sched_cfs fs \
    myfs lwext4_rs net dns display rtc bus-mmio bus-pci driver-ramdisk \
    driver-ixgbe driver-fxmac driver-bcm2835-sdhci log-level-off \
    log-level-error log-level-warn log-level-info log-level-debug log-level-trace

# --- 确定内核特性前缀 ---
ifeq ($(NO_AXSTD), y)
    _KERNEL_FEAT_PREFIX := axfeat/
else
    _KERNEL_FEAT_PREFIX := axstd/
endif

# ------------------------------------------------------------------------------
# Part 2: 特性聚合、分拣与组装 (The Original Way, Adapted)
# ------------------------------------------------------------------------------
# --- 1. 聚合所有特性到一个临时的、可能会被污染的列表 ---
_all_features_raw :=
_all_features_raw += $(FEATURES)
ifeq ($(BUS), mmio)
    _all_features_raw += bus-mmio
endif
ifneq ($(SMP), 1)
ifneq ($(SMP), 0)
    _all_features_raw += smp
endif
endif

# --- 2. 清理和规范化这个列表 ---
_all_features_clean := $(strip $(shell echo $(_all_features_raw) | tr ',' ' '))

# --- 3. 执行分拣 ---
_kernel_features   := $(filter $(_KERNEL_FEATURE_LIST), $(_all_features_clean))
_toplevel_features := $(filter-out $(_KERNEL_FEATURE_LIST), $(_all_features_clean))

# --- 4. 组装带前缀的内核特性 ---
_prefixed_kernel_features := $(addprefix $(_KERNEL_FEAT_PREFIX), $(_kernel_features))

# --- 5. 合并所有特性到一个最终的、空格分隔的列表 ---
_final_feature_list := $(_prefixed_kernel_features) $(_toplevel_features)

# --- 6. 最终格式化为 CSV ---
FINAL_FEATURES := $(shell echo $(strip $(_final_feature_list)) | tr ' ' ',')

# ------------------------------------------------------------------------------
# Part 3: 调试与清理
# ------------------------------------------------------------------------------
# $(info --- DEBUG features.mk ---)
# $(info All features collected (clean): [$(_all_features_clean)])
# $(info Kernel Features to prefix: [$(_kernel_features)])
# $(info Toplevel Features: [$(_toplevel_features)])
# $(info Final feature string for Cargo: [$(FINAL_FEATURES)])
# $(info -------------------------)

# 清理临时变量
_KERNEL_FEATURE_LIST  :=
_KERNEL_FEAT_PREFIX   :=
_all_features_raw     :=
_all_features_clean   :=
_kernel_features      :=
_toplevel_features    :=
_prefixed_kernel_features :=
_final_feature_list   :=
# /starry/.arceos/scripts/make/cargo.mk (Polished Version)

# --- Verbosity Level ---
VERBOSE_FLAG.1 := -v
VERBOSE_FLAG.2 := -vv
verbose := $(VERBOSE_FLAG.$(V))

# --- Build Arguments ---
build_args-release := --release
build_args-debug   := # Debug is default, no flag needed

# 组装基础构建参数
build_args := \
  -Z unstable-options \
  --target $(TARGET) \
  --target-dir $(TARGET_DIR) \
  $(build_args-$(MODE)) \
  $(verbose)

# --- Rust Build & Linker Flags ---
# 基础 RUSTFLAGS，总是需要
RUSTFLAGS := \
    -A unsafe_op_in_unsafe_fn \
    -C code-model=kernel

# 链接器专属参数，只在需要时由 build.mk 添加
RUSTFLAGS_LINK_ARGS := \
    -C link-arg=-T$(LD_SCRIPT) \
    -C link-arg=-no-pie \
    -C link-arg=-znostart-stop-gc

# 文档生成标志
RUSTDOCFLAGS := \
    -Z unstable-options \
    --enable-index-page \
    -D rustdoc::broken_intra_doc_links

ifeq ($(filter doc_check_missing,$(MAKECMDGOALS)),doc_check_missing)
  RUSTDOCFLAGS += -D missing-docs
endif

# ==============================================================================
# Cargo Command Macros
# ==============================================================================

# --- cargo_build macro ---
# $(1): The package NAME to build (e.g., "arceos-main", "axlibc").
# $(2): The list of features, comma-separated.
define cargo_build
	$(call run_cmd,cargo build \
		-p $(1) \
		$(build_args) \
		$(if $(2),--features "$(strip $(2))") \
	)
endef

# --- cargo_clippy macro ---
clippy_args := -A clippy::new_without_default -A unsafe_op_in_unsafe_fn
define cargo_clippy
  # TODO: Refactor to avoid hardcoding 'axlog'
  $(call run_cmd,cargo clippy,--all-features --workspace --exclude axlog $(1) $(verbose) -- $(clippy_args))
  $(call run_cmd,cargo clippy,-p axlog $(1) $(verbose) -- $(clippy_args))
endef

# --- cargo_doc macro ---
all_packages := \
  $(shell ls $(CURDIR)/modules) \
  axfeat arceos_api axstd axlibc
define cargo_doc
  $(call run_cmd,cargo doc,--no-deps --all-features --workspace --exclude "arceos-*" $(verbose))
  @# run twice to fix broken hyperlinks
  $(foreach p,$(all_packages), \
    $(call run_cmd,cargo rustdoc,--all-features -p $(p) $(verbose))
  )
endef

# --- unit_test macro ---
define unit_test
  # TODO: Refactor to avoid hardcoding 'axfs'
  $(call run_cmd,cargo test,-p axfs $(1) $(verbose) -- --nocapture)
  $(call run_cmd,cargo test,-p axfs $(1) --features "myfs" $(verbose) -- --nocapture)
  $(call run_cmd,cargo test,--workspace --exclude axfs $(1) $(verbose) -- --nocapture)
endef
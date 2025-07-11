# Main building script (Final, Logically Corrected Version)

include scripts/make/cargo.mk

# --- Prepare build flags ---
ifeq ($(APP_TYPE), c)
  # ...
else
  # _rust_pkg_raw := $(shell sed -n 's/^name = "\([^"]*\)".*/\1/p' $(APP)/Cargo.toml | head -n 1)
  # rust_package := $(strip $(_rust_pkg_raw))
  rust_package := $(shell basename $(APP))
  rust_elf := $(TARGET_DIR)/$(TARGET)/$(MODE)/$(rust_package)
endif

ifneq ($(filter $(MAKECMDGOALS),doc doc_check_missing),)
  $(if $(V), $(info RUSTFLAGS: "$(RUSTFLAGS)") $(info RUSTDOCFLAGS: "$(RUSTDOCFLAGS)"))
  export RUSTFLAGS
  export RUSTDOCFLAGS
else ifneq ($(filter $(MAKECMDGOALS),unittest unittest_no_fail_fast),)
  # run `make unittest`
  $(if $(V), $(info RUSTFLAGS: "$(RUSTFLAGS)"))
  export RUSTFLAGS
else ifeq ($(filter $(MAKECMDGOALS),defconfig oldconfig clippy),)
  ifneq ($(V),)
    $(info APP: "$(APP)")
    $(info APP_TYPE: "$(APP_TYPE)")
    $(info FEATURES: "$(FEATURES)")
    $(info arceos features: "$(AX_FEAT)")
    $(info lib features: "$(LIB_FEAT)")
    $(info app features: "$(APP_FEAT)")
  endif
  ifeq ($(APP_TYPE), c)
    $(if $(V), $(info CFLAGS: "$(CFLAGS)") $(info LDFLAGS: "$(LDFLAGS)"))
  else ifeq ($(APP_TYPE), rust)
    RUSTFLAGS += $(RUSTFLAGS_LINK_ARGS)
  endif
  $(if $(V), $(info RUSTFLAGS: "$(RUSTFLAGS)"))
  export RUSTFLAGS
endif

# ==============================================================================
# Part 2: Corrected Build Targets
# ==============================================================================
# Phony target for the main cargo build step. It only runs `cargo build`.
.PHONY: _cargo_build
_cargo_build: oldconfig
	@printf "    $(GREEN_C)Building$(END_C) App: $(APP_NAME), Arch: $(ARCH), Platform: $(PLAT_NAME), App type: $(APP_TYPE)\n"
ifeq ($(APP_TYPE), rust)
	$(call cargo_build,$(APP),$(AX_FEAT) $(LIB_FEAT) $(APP_FEAT))
else ifeq ($(APP_TYPE), c)
	$(call cargo_build,ulib/axlibc,$(AX_FEAT) $(LIB_FEAT))
endif

$(OUT_ELF): _cargo_build
	@echo "    $(GREEN_C)Copying$(END_C) ELF from target directory..."
	@mkdir -p $(dir $@)
	@echo "DEBUG: Source ELF path is [$(rust_elf)]"
	@echo "DEBUG: Listing contents of the release directory:"
	@cp $(rust_elf) $@

$(OUT_BIN): $(OUT_ELF)
	@echo "    $(GREEN_C)Creating$(END_C) Binary from ELF..."
	$(call run_cmd,$(OBJCOPY),$< --strip-all -O binary $@)

# Rule for creating U-Boot image (if needed)
ifeq ($(ARCH), aarch64)
  uimg_arch := arm64
else ifeq ($(ARCH), riscv64)
  uimg_arch := riscv
else
  uimg_arch := $(ARCH)
endif

$(OUT_UIMG): $(OUT_BIN)
	@echo "    $(GREEN_C)Creating$(END_C) U-Boot Image..."
	$(call run_cmd,mkimage,\
		-A $(uimg_arch) -O linux -T kernel -C none \
		-a $(subst _,,$(shell axconfig-gen "$(OUT_CONFIG)" -r plat.kernel-base-paddr)) \
		-d $< $@)

# Rule to create the output directory
$(OUT_DIR):
	$(call run_cmd,mkdir,-p $@)
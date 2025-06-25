# ==============================================================================
# Corrected QEMU Arguments Makefile
# ==============================================================================

QEMU := qemu-system-$(ARCH)

ifeq ($(BUS), mmio)
  vdev-suffix := device
else ifeq ($(BUS), pci)
  vdev-suffix := pci
else
  $(error "BUS" must be one of "mmio" or "pci")
endif

ifeq ($(ARCH), x86_64)
  machine := q35
else ifeq ($(ARCH), riscv64)
  machine := virt
else ifeq ($(ARCH), aarch64)
  ifeq ($(PLAT_NAME), aarch64-raspi4)
    machine := raspi4b
    override MEM := 2G
  else
    machine := virt
  endif
else ifeq ($(ARCH), loongarch64)
  machine := virt
  override MEM := 1G
endif

qemu_args-x86_64 := \
  -machine $(machine) \
  -kernel $(OUT_ELF)

qemu_args-riscv64 := \
  -machine $(machine) \
  -bios default \
  -kernel $(FINAL_IMG)

qemu_args-aarch64 := \
  -cpu cortex-a72 \
  -machine $(machine) \
  -kernel $(FINAL_IMG)

qemu_args-loongarch64 := \
  -machine $(machine) \
  -kernel $(OUT_ELF)

# Base arguments, always enabled
qemu_args-y := -m $(MEM) -smp $(SMP) $(qemu_args-$(ARCH))

# --- Storage Device Arguments (Conditional) ---
qemu_args-$(BLK) += \
  -device virtio-blk-$(vdev-suffix),drive=disk0 \
  -drive id=disk0,if=none,format=raw,file=$(DISK_IMG)

# --- Network Device Arguments (Conditional) ---
# This block is now wrapped to ensure it only runs when NET=y
ifeq ($(NET), y)
  # First, add the virtio-net device itself
  qemu_args-y += -device virtio-net-$(vdev-suffix),netdev=net0

  # Then, configure the network backend based on NET_DEV
  ifeq ($(NET_DEV), user)
    HOSTFWD ?= n
    net_user_args := -netdev user,id=net0
    ifeq ($(HOSTFWD), y)
      net_user_args += ,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555
    endif
    qemu_args-y += $(net_user_args)
  else ifeq ($(NET_DEV), tap)
    qemu_args-y += -netdev tap,id=net0,script=scripts/net/qemu-ifup.sh,downscript=no,vhost=$(VHOST),vhostforce=$(VHOST)
    QEMU := sudo $(QEMU)
  else ifeq ($(NET_DEV), bridge)
    qemu_args-y += -netdev bridge,id=net0,br=virbr0
    QEMU := sudo $(QEMU)
  else
    # This error will now only trigger if NET=y but NET_DEV is invalid.
    $(error "NET_DEV" must be one of "user", "tap", or "bridge")
  endif

  # Optional network dump
  ifeq ($(NET_DUMP), y)
    qemu_args-y += -object filter-dump,id=dump0,netdev=net0,file=netdump.pcap
  endif
endif

# --- VFIO Passthrough ---
ifneq ($(VFIO_PCI),)
  qemu_args-y += --device vfio-pci,host=$(VFIO_PCI)
  QEMU := sudo $(QEMU)
endif

# --- Graphics Arguments ---
ifeq ($(GRAPHIC), y)
  qemu_args-y += \
    -device virtio-gpu-$(vdev-suffix) -vga none \
    -serial mon:stdio
else
  # if no graphic, use nographic mode
  qemu_args-y += -nographic
endif

# --- QEMU Logging ---
ifeq ($(QEMU_LOG), y)
  qemu_args-y += -D qemu.log -d in_asm,int,mmu,pcall,cpu_reset,guest_errors
endif

# --- Hardware Acceleration ---
ifeq ($(ACCEL),)
  ifneq ($(findstring -microsoft, $(shell uname -r | tr '[:upper:]' '[:lower:]')),)
    ACCEL := n
  else ifeq ($(ARCH), x86_64)
    ACCEL := $(if $(findstring x86_64, $(shell uname -m)),y,n)
  else ifeq ($(ARCH), aarch64)
    ACCEL := $(if $(findstring aarch64, $(shell uname -m)),y,n) # Corrected from arm64
  else
    ACCEL := n
  endif
endif

ifeq ($(shell uname), Darwin)
  qemu_args-$(ACCEL) += -cpu host -accel hvf
else ifneq ($(wildcard /dev/kvm),)
  # Do not use KVM for debugging as it can interfere with GDB
  ifeq ($(MAKECMDGOALS), debug)
    # KVM disabled for debug
  else
    qemu_args-$(ACCEL) += -cpu host -accel kvm
  endif
endif

# --- Final argument sets for run and debug ---
qemu_args_run := $(qemu_args-y) $(qemu_args-$(ACCEL))
qemu_args_debug := $(qemu_args-y) -s -S # Note: KVM is implicitly disabled for debug

# --- Re-usable run commands ---
define run_qemu
  @printf "    $(CYAN_C)Running$(END_C) on qemu...\n"
  $(call run_cmd,$(QEMU),$(qemu_args_run))
endef

define run_qemu_debug
  @printf "    $(CYAN_C)Debugging$(END_C) on qemu...\n"
  $(call run_cmd,$(QEMU),$(qemu_args_debug))
endef
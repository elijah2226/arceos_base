# /starry/.arceos/scripts/make/qemu.mk (Polished and Data-Driven Version)

# ==============================================================================
# Part 1: 基础配置 (Base Configuration)
# ==============================================================================

# QEMU := qemu-system-$(ARCH)
QEMU := /usr/bin/qemu-system-$(ARCH)

# --- 设备后缀 ---
VDEV_SUFFIX.pci  := pci
VDEV_SUFFIX.mmio := device
vdev-suffix := $(VDEV_SUFFIX.$(BUS))
# 如果 BUS 变量无效，则报错
ifneq ($(filter pci mmio,$(BUS)),$(BUS))
  $(error "BUS must be one of 'pci' or 'mmio', but is '$(BUS)'")
endif

# ==============================================================================
# Part 2: 架构相关配置 (Architecture-Specific Configuration)
# ==============================================================================

# --- 默认 Machine 类型 ---
QEMU_MACHINE.x86_64      := q35
QEMU_MACHINE.riscv64     := virt
QEMU_MACHINE.aarch64     := virt
QEMU_MACHINE.loongarch64 := virt
# 特例: aarch64 raspi4
ifeq ($(PLAT_NAME), aarch64-raspi4)
    QEMU_MACHINE.aarch64 := raspi4b
    override MEM := 2G
endif
# 特例: loongarch64
ifeq ($(ARCH), loongarch64)
    override MEM := 1G
endif

# --- 架构专属参数 ---
QEMU_ARCH_ARGS.x86_64      := -machine $(QEMU_MACHINE.$(ARCH)) -kernel $(OUT_ELF)
QEMU_ARCH_ARGS.riscv64     := -machine $(QEMU_MACHINE.$(ARCH)) -bios default -kernel $(FINAL_IMG)
QEMU_ARCH_ARGS.aarch64     := -machine $(QEMU_MACHINE.$(ARCH)) -cpu cortex-a72 -kernel $(FINAL_IMG)
QEMU_ARCH_ARGS.loongarch64 := -machine $(QEMU_MACHINE.$(ARCH)) -kernel $(OUT_ELF)

# ==============================================================================
# Part 3: 参数组装 (Argument Assembly)
# ==============================================================================

# --- 基础参数 ---
qemu_args := -m $(MEM) -smp $(SMP) $(QEMU_ARCH_ARGS.$(ARCH))

# --- 存储设备 ---
ifeq ($(BLK),y)
  qemu_args += -device virtio-blk-$(vdev-suffix),drive=disk0
  qemu_args += -drive id=disk0,if=none,format=raw,file=$(DISK_IMG)
endif

# --- 网络设备 ---
ifeq ($(NET),y)
  qemu_args += -device virtio-net-$(vdev-suffix),netdev=net0

  # --- 网络后端配置 (使用 ifeq，更清晰) ---
  
  # -- User Mode --
  # 先定义基础部分
  net_user_args := -netdev user,id=net0
  # 如果 HOSTFWD=y，则追加 hostfwd 参数，注意前面的逗号
  ifeq ($(HOSTFWD),y)
    net_user_args += ,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555
  endif
  NET_BACKEND.user := $(net_user_args)

  # -- Tap Mode --
  NET_BACKEND.tap     := -netdev tap,id=net0,script=scripts/net/qemu-ifup.sh,downscript=no,vhost=$(VHOST),vhostforce=$(VHOST)
  # -- Bridge Mode --
  NET_BACKEND.bridge  := -netdev bridge,id=net0,br=virbr0

  # --- 应用后端配置并检查错误 ---
  qemu_args += $(NET_BACKEND.$(NET_DEV))
  ifneq ($(filter user tap bridge,$(NET_DEV)),$(NET_DEV))
      $(error "NET_DEV must be one of 'user', 'tap', or 'bridge' when NET=y, but is '$(NET_DEV)'")
  endif

  # --- Sudo 权限 ---
  # 只在 tap 或 bridge 模式下需要 sudo
  ifneq ($(filter tap bridge,$(NET_DEV)),)
      QEMU := sudo $(QEMU)
  endif

  # --- 网络抓包 ---
  ifeq ($(NET_DUMP),y)
    qemu_args += -object filter-dump,id=dump0,netdev=net0,file=netdump.pcap
  endif
endif

# --- VFIO 直通 ---
ifneq ($(VFIO_PCI),)
  qemu_args += --device vfio-pci,host=$(VFIO_PCI)
  QEMU := sudo $(QEMU)
endif

# --- 图形界面 (最终 VNC 方案) ---
# 强制 QEMU 启动一个 VNC 服务器。
# 0.0.0.0 表示监听容器的所有网络接口。
# :1 表示在 1 号显示器上 (对应 TCP 端口 5901)。
qemu_args += -vnc 0.0.0.0:1
qemu_args += -serial stdio

# --- QEMU 日志 ---
ifeq ($(QEMU_LOG),y)
  qemu_args += -D qemu.log -d in_asm,int,mmu,pcall,cpu_reset,guest_errors
endif

# --- 硬件加速 ---
# 自动检测是否启用加速
ifeq ($(ACCEL),)
  # 在 WSL1 上禁用
  ifneq ($(findstring -microsoft, $(shell uname -r | tr '[:upper:]' '[:lower:]')),)
    ACCEL := n
  # 如果主机与客户机架构匹配，则启用
  else ifeq ($(ARCH),$(shell uname -m))
    ACCEL := y
  else
    ACCEL := n
  endif
endif

# 根据系统和场景选择加速器
# 只有在 ACCEL=y 且非调试场景下才添加加速参数
qemu_accel_args :=
ifeq ($(ACCEL),y)
  ifeq ($(BUILD_SCENARIO),debug)
    # KVM/HVF 在 GDB 调试下可能导致问题，默认禁用
  else
    ifeq ($(shell uname), Darwin)
      qemu_accel_args := -cpu host -accel hvf
    else ifneq ($(wildcard /dev/kvm),)
      qemu_accel_args := -cpu host -accel kvm
    endif
  endif
endif

# ==============================================================================
# Part 4: 最终命令定义 (Final Command Definitions)
# ==============================================================================

# --- 最终参数集 ---
qemu_args_run   := $(qemu_args) $(qemu_accel_args)
qemu_args_debug := $(qemu_args) -s -S # -S for GDB, KVM is implicitly disabled

# --- 可重用命令 ---
# define run_qemu
#   @printf "    $(CYAN_C)Running$(END_C) on qemu...\n"
#   $(call run_cmd,$(QEMU),$(qemu_args_run))
# endef

define run_qemu
	@printf "    $(CYAN_C)Running$(END_C) on qemu...\n"
	@printf "    $(CYAN_C)QEMU is now a VNC server. Connect from your Windows VNC client via the container's IP and port 5901.$(END_C)\n"
	$(QEMU) $(qemu_args_run)
endef

# define run_qemu_debug
#   @printf "    $(CYAN_C)Debugging$(END_C) on qemu...\n"
#   $(call run_cmd,$(QEMU),$(qemu_args_debug))
# endef

define run_qemu_debug
  @printf "    $(CYAN_C)Debugging$(END_C) on qemu...\n"
  @printf "    $(CYAN_C)QEMU is now a VNC server. Connect from your Windows VNC client.$(END_C)\n"
  $(QEMU) $(qemu_args_debug)
endef
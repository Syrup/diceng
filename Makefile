BINARY_NAME  := diceng
DIST_DIR     := dist

# Android NDK
NDK_VERSION  := r27c
NDK_DIR      := $(HOME)/android-ndk-$(NDK_VERSION)
NDK_ZIP      := android-ndk-$(NDK_VERSION)-linux.zip
NDK_URL      := https://dl.google.com/android/repository/$(NDK_ZIP)
API_LEVEL    := 24
TOOLCHAIN    := $(NDK_DIR)/toolchains/llvm/prebuilt/linux-x86_64
STRIP        := $(TOOLCHAIN)/bin/llvm-strip

# Platform: android, linux, macos, windows
PLATFORM ?= linux

# Arch per platform
ANDROID_ARCH ?= aarch64
LINUX_ARCH   ?= x86_64
MACOS_ARCH   ?= aarch64
WINDOWS_ARCH ?= x86_64

.PHONY: help build build-all strip release install check clean \
	ensure-target check-deps setup-ndk build-so \
	_build-android _build-linux _build-macos _build-windows \
	android-install

help:
	@echo "diceng — Multi-platform Build"
	@echo ""
	@echo "Usage:"
	@echo "  make build [PLATFORM=...] [ARCH=...]   Build for target platform"
	@echo "  make build-all                          Build all platforms"
	@echo "  make build-so                           Build shared library (.so) for FFI"
	@echo "  make strip                              Strip debug symbols from dist/"
	@echo "  make release [PLATFORM=...] [ARCH=...]  Build + strip"
	@echo "  make android-install                    Push Android build via ADB"
	@echo "  make install                            Install binary locally"
	@echo "  make check                              Run fmt + clippy + test"
	@echo "  make clean                              Clean build artifacts"
	@echo ""
	@echo "Platforms & Arches:"
	@echo "  make build PLATFORM=android [ANDROID_ARCH=aarch64|armv7|x86_64|i686]"
	@echo "  make build PLATFORM=linux   [LINUX_ARCH=x86_64|aarch64|x86_64-musl|aarch64-musl|armv7]"
	@echo "  make build PLATFORM=macos   [MACOS_ARCH=aarch64|x86_64]"
	@echo "  make build PLATFORM=windows [WINDOWS_ARCH=x86_64|i686|aarch64|x86_64-gnu]"
	@echo ""
	@echo "Examples:"
	@echo "  make build                                       # Linux x86_64 (default)"
	@echo "  make build PLATFORM=android ANDROID_ARCH=armv7   # Android ARM32"
	@echo "  make build PLATFORM=linux LINUX_ARCH=aarch64-musl # Linux ARM64 static"
	@echo "  make build PLATFORM=macos                        # macOS Apple Silicon"

# ── Ensure Rust Target Installed ───────────────
ensure-target:
	@rustup target list --installed 2>/dev/null | grep -q $(TARGET_TRIPLE) || { \
		echo "Error: Rust target '$(TARGET_TRIPLE)' not installed."; \
		echo ""; \
		echo "Install with:"; \
		echo "  rustup target add $(TARGET_TRIPLE)"; \
		echo ""; \
		exit 1; \
	}

# ── Check Dependencies (Android) ───────────────
check-deps:
	@command -v cargo >/dev/null 2>&1 || { echo "Error: cargo not found. Install Rust first."; exit 1; }
	@./scripts/prompt-install.sh \
		"cargo-ndk" \
		"command -v cargo-ndk" \
		"cargo install cargo-ndk" \
		"  cargo install cargo-ndk"

setup-ndk:
	@./scripts/prompt-install.sh \
		"Android NDK $(NDK_VERSION)" \
		"test -d $(NDK_DIR)" \
		"curl -L --progress-bar '$(NDK_URL)' -o /tmp/$(NDK_ZIP) && unzip -q /tmp/$(NDK_ZIP) -d $(HOME) && rm /tmp/$(NDK_ZIP)" \
		"  curl -L --progress-bar '$(NDK_URL)' -o /tmp/$(NDK_ZIP)\n  unzip -q /tmp/$(NDK_ZIP) -d $(HOME)\n  rm /tmp/$(NDK_ZIP)"

# ── Build Dispatch ─────────────────────────────
build:
ifeq ($(PLATFORM),android)
	@$(MAKE) --no-print-directory _build-android
else ifeq ($(PLATFORM),linux)
	@$(MAKE) --no-print-directory _build-linux
else ifeq ($(PLATFORM),macos)
	@$(MAKE) --no-print-directory _build-macos
else ifeq ($(PLATFORM),windows)
	@$(MAKE) --no-print-directory _build-windows
else
	$(error Unknown platform '$(PLATFORM)'. Use: android, linux, macos, windows)
endif

# ── Android ────────────────────────────────────
_build-android: check-deps setup-ndk
	@case $(ANDROID_ARCH) in \
		aarch64) TRIPLE=aarch64-linux-android; ABI=arm64-v8a;; \
		armv7)   TRIPLE=armv7-linux-androideabi; ABI=armeabi-v7a;; \
		x86_64)  TRIPLE=x86_64-linux-android; ABI=x86_64;; \
		i686)    TRIPLE=i686-linux-android; ABI=x86;; \
		*) echo "Error: Unknown Android arch '$(ANDROID_ARCH)'"; exit 1;; \
	esac; \
	echo "Building $(BINARY_NAME) for Android $$TRIPLE ($$ABI)..."; \
	ANDROID_NDK_HOME=$(NDK_DIR) cargo ndk \
		-t $$ABI \
		--platform $(API_LEVEL) \
		build --release --bin $(BINARY_NAME); \
	mkdir -p $(DIST_DIR)/android-$(ANDROID_ARCH); \
	cp target/$$TRIPLE/release/$(BINARY_NAME) $(DIST_DIR)/android-$(ANDROID_ARCH)/; \
	echo ""; \
	echo "Done: $(DIST_DIR)/android-$(ANDROID_ARCH)/$(BINARY_NAME)"; \
	du -h $(DIST_DIR)/android-$(ANDROID_ARCH)/$(BINARY_NAME)

# ── Linux ──────────────────────────────────────
_build-linux:
	@case $(LINUX_ARCH) in \
		x86_64)      TRIPLE=x86_64-unknown-linux-gnu;; \
		aarch64)     TRIPLE=aarch64-unknown-linux-gnu;; \
		x86_64-musl) TRIPLE=x86_64-unknown-linux-musl;; \
		aarch64-musl) TRIPLE=aarch64-unknown-linux-musl;; \
		armv7)       TRIPLE=armv7-unknown-linux-gnueabihf;; \
		*) echo "Error: Unknown Linux arch '$(LINUX_ARCH)'"; exit 1;; \
	esac; \
	TARGET_TRIPLE=$$TRIPLE $(MAKE) --no-print-directory ensure-target || exit 1; \
	echo "Building $(BINARY_NAME) for Linux $(LINUX_ARCH) ($$TRIPLE)..."; \
	cargo build --release --target $$TRIPLE --bin $(BINARY_NAME); \
	mkdir -p $(DIST_DIR)/linux-$(LINUX_ARCH); \
	cp target/$$TRIPLE/release/$(BINARY_NAME) $(DIST_DIR)/linux-$(LINUX_ARCH)/; \
	echo ""; \
	echo "Done: $(DIST_DIR)/linux-$(LINUX_ARCH)/$(BINARY_NAME)"; \
	du -h $(DIST_DIR)/linux-$(LINUX_ARCH)/$(BINARY_NAME)

# ── macOS ──────────────────────────────────────
_build-macos:
	@case $(MACOS_ARCH) in \
		x86_64)  TRIPLE=x86_64-apple-darwin;; \
		aarch64) TRIPLE=aarch64-apple-darwin;; \
		*) echo "Error: Unknown macOS arch '$(MACOS_ARCH)'"; exit 1;; \
	esac; \
	TARGET_TRIPLE=$$TRIPLE $(MAKE) --no-print-directory ensure-target || exit 1; \
	echo "Building $(BINARY_NAME) for macOS $(MACOS_ARCH) ($$TRIPLE)..."; \
	cargo build --release --target $$TRIPLE --bin $(BINARY_NAME); \
	mkdir -p $(DIST_DIR)/macos-$(MACOS_ARCH); \
	cp target/$$TRIPLE/release/$(BINARY_NAME) $(DIST_DIR)/macos-$(MACOS_ARCH)/; \
	echo ""; \
	echo "Done: $(DIST_DIR)/macos-$(MACOS_ARCH)/$(BINARY_NAME)"; \
	du -h $(DIST_DIR)/macos-$(MACOS_ARCH)/$(BINARY_NAME)

# ── Windows ────────────────────────────────────
_build-windows:
	@case $(WINDOWS_ARCH) in \
		x86_64)     TRIPLE=x86_64-pc-windows-msvc; EXT=.exe;; \
		i686)       TRIPLE=i686-pc-windows-msvc; EXT=.exe;; \
		aarch64)    TRIPLE=aarch64-pc-windows-msvc; EXT=.exe;; \
		x86_64-gnu) TRIPLE=x86_64-pc-windows-gnu; EXT=.exe;; \
		*) echo "Error: Unknown Windows arch '$(WINDOWS_ARCH)'"; exit 1;; \
	esac; \
	TARGET_TRIPLE=$$TRIPLE $(MAKE) --no-print-directory ensure-target || exit 1; \
	echo "Building $(BINARY_NAME) for Windows $(WINDOWS_ARCH) ($$TRIPLE)..."; \
	cargo build --release --target $$TRIPLE --bin $(BINARY_NAME); \
	mkdir -p $(DIST_DIR)/windows-$(WINDOWS_ARCH); \
	cp target/$$TRIPLE/release/$(BINARY_NAME)$$EXT $(DIST_DIR)/windows-$(WINDOWS_ARCH)/; \
	echo ""; \
	echo "Done: $(DIST_DIR)/windows-$(WINDOWS_ARCH)/$(BINARY_NAME)$$EXT"; \
	du -h $(DIST_DIR)/windows-$(WINDOWS_ARCH)/$(BINARY_NAME)$$EXT

# ── Shared Library (.so) for FFI ──────────────
build-so:
	@echo "Building libdiceng.so (cdylib)...";
	cargo build --release --lib;
	mkdir -p $(DIST_DIR)/linux-x86_64;
	cp target/release/libdiceng.so $(DIST_DIR)/linux-x86_64/;
	strip $(DIST_DIR)/linux-x86_64/libdiceng.so;
	echo ""; \
	echo "Done: $(DIST_DIR)/linux-x86_64/libdiceng.so"; \
	du -h $(DIST_DIR)/linux-x86_64/libdiceng.so

# ── Build All ──────────────────────────────────
build-all:
	@echo "=== Building all platforms ==="
	@echo ""
	@for arch in aarch64 armv7 x86_64 i686; do \
		$(MAKE) build PLATFORM=android ANDROID_ARCH=$$arch || exit 1; \
		echo ""; \
	done
	@for arch in x86_64 aarch64 x86_64-musl aarch64-musl armv7; do \
		$(MAKE) build PLATFORM=linux LINUX_ARCH=$$arch || exit 1; \
		echo ""; \
	done
	@for arch in aarch64 x86_64; do \
		$(MAKE) build PLATFORM=macos MACOS_ARCH=$$arch || exit 1; \
		echo ""; \
	done
	@for arch in x86_64 i686 aarch64 x86_64-gnu; do \
		$(MAKE) build PLATFORM=windows WINDOWS_ARCH=$$arch || exit 1; \
		echo ""; \
	done
	@echo "=== All builds complete: $(DIST_DIR)/ ==="
	@ls -lh $(DIST_DIR)/*/

# ── Strip ──────────────────────────────────────
strip:
	@echo "Stripping debug symbols..."
	@for bin in $(DIST_DIR)/*/$(BINARY_NAME) $(DIST_DIR)/*/$(BINARY_NAME).exe; do \
		[ -f "$$bin" ] || continue; \
		echo "  $$bin"; \
		strip "$$bin" 2>/dev/null || true; \
	done
	@echo "Done."

# ── Release (build + strip) ────────────────────
release:
	@$(MAKE) build PLATFORM=$(PLATFORM) ANDROID_ARCH=$(ANDROID_ARCH) LINUX_ARCH=$(LINUX_ARCH) MACOS_ARCH=$(MACOS_ARCH) WINDOWS_ARCH=$(WINDOWS_ARCH)
	@$(MAKE) strip

# ── Android Install ────────────────────────────
android-install:
	@$(MAKE) build PLATFORM=android
	@echo "Pushing to device via ADB..."
	adb push $(DIST_DIR)/android-$(ANDROID_ARCH)/$(BINARY_NAME) /data/local/tmp/
	adb shell chmod +x /data/local/tmp/$(BINARY_NAME)
	@echo ""
	@echo "Run on device:"
	@echo "  adb shell /data/local/tmp/$(BINARY_NAME) --help"

# ── Local Install ──────────────────────────────
install:
	cargo install --path .

# ── Check (fmt + clippy + test) ────────────────
check:
	cargo fmt --check
	cargo clippy -- -D warnings
	cargo test

# ── Clean ──────────────────────────────────────
clean:
	cargo clean
	rm -rf $(DIST_DIR)

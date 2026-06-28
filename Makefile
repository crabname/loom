.PHONY: help check build run icons bundle dmg clean

APP_NAME := Loom
TARGET_TRIPLE := $(shell rustc -vV | awk '/^host: / { print $$2 }')
RELEASE_APP := target/$(TARGET_TRIPLE)/release/bundle/osx/$(APP_NAME).app
ARCH_SUFFIX := $(shell uname -m | sed 's/arm64/aarch64/; s/x86_64/x86_64/')
RELEASE_DMG := target/$(TARGET_TRIPLE)/release/$(APP_NAME)-$(ARCH_SUFFIX).dmg
ICON_SOURCE := assets/icon@2x.png

help:
	@echo "Targets:"
	@echo "  make check          Compile-check the project"
	@echo "  make build          Build release binary"
	@echo "  make run            Run the app in debug mode"
	@echo "  make icons          Regenerate assets/AppIcon.icns from PNG"
	@echo "  make bundle         Build macOS .app bundle (release)"
	@echo "  make dmg            Build macOS .app and .dmg (release)"
	@echo "  make clean          Remove build artifacts"

check:
	cargo check

build:
	cargo build --release

run:
	cargo run

icons:
	@test -f "$(ICON_SOURCE)" || (echo "Missing $(ICON_SOURCE)" && exit 1)
	rm -rf assets/icon.iconset
	mkdir -p assets/icon.iconset
	@for size in 16 32 128 256 512; do \
		sips -z $$size $$size "$(ICON_SOURCE)" --out "assets/icon.iconset/icon_$${size}x$${size}.png" >/dev/null; \
		double=$$((size * 2)); \
		sips -z $$double $$double "$(ICON_SOURCE)" --out "assets/icon.iconset/icon_$${size}x$${size}@2x.png" >/dev/null; \
	done
	iconutil -c icns assets/icon.iconset -o assets/AppIcon.icns
	rm -rf assets/icon.iconset

bundle: icons
	./script/bundle-mac

dmg: icons
	./script/bundle-mac -D
	@test -f "$(RELEASE_DMG)" || (echo "Expected DMG at $(RELEASE_DMG)" && exit 1)
	@echo "$(RELEASE_DMG)"

clean:
	cargo clean

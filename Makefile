AGENT_VERSION = 0.1.0
SETUP_VERSION = 0.1.0

.PHONY: \
    build-agent-aarch64-unknown-linux-musl \
	build-agent-x86_64-unknown-linux-musl \
	build-agent-x86_64-apple-darwin \
	build-agent-aarch64-apple-darwin \
	build-agent-x86_64-pc-windows-msvc \
    build-setup-aarch64-unknown-linux-musl \
	build-setup-x86_64-unknown-linux-musl \
	build-setup-x86_64-apple-darwin \
	build-setup-aarch64-apple-darwin \
	build-setup-x86_64-pc-windows-msvc \
	agent-version \
    setup-version

.EXPORT_ALL_VARIABLES:

agent-version:
	@echo $(AGENT_VERSION)

setup-version:
	@echo $(SETUP_VERSION)

build-agent-x86_64-unknown-linux-musl: export TARGET_CC = cc
build-agent-x86_64-unknown-linux-musl: export TARGET_AR = ar
build-agent-x86_64-unknown-linux-musl: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	strip build/bin/agent
	mv build/bin/agent build/cluvio-agent
	tar caf dist/agent-$(AGENT_VERSION)-x86_64-unknown-linux-musl.tar.xz -C build/ cluvio-agent

build-agent-aarch64-unknown-linux-musl: export TARGET_CC = aarch64-linux-gnu-gcc
build-agent-aarch64-unknown-linux-musl: export TARGET_AR = aarch64-linux-gnu-ar
build-agent-aarch64-unknown-linux-musl: clean
	mkdir -p build dist
	cargo install \
		--target aarch64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	aarch64-linux-gnu-strip build/bin/agent
	mv build/bin/agent build/cluvio-agent
	tar caf dist/agent-$(AGENT_VERSION)-aarch64-unknown-linux-musl.tar.xz -C build/ cluvio-agent

build-agent-x86_64-apple-darwin: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	strip build/bin/agent
	mv build/bin/agent build/cluvio-agent
	scripts/apple-codesign.sh build/cluvio-agent
	scripts/apple-notarize.sh build/cluvio-agent
	tar caf dist/agent-$(AGENT_VERSION)-x86_64-apple-darwin.tar.xz -C build/ cluvio-agent

build-agent-aarch64-apple-darwin: export SDKROOT = $(shell xcrun -sdk macosx11.1 --show-sdk-path)
build-agent-aarch64-apple-darwin: export MACOSX_DEPLOYMENT_TARGET = $(shell xcrun -sdk macosx11.1 --show-sdk-platform-version)
build-agent-aarch64-apple-darwin: clean
	mkdir -p build dist
	cargo install \
		--target aarch64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	strip build/bin/agent
	mv build/bin/agent build/cluvio-agent
	scripts/apple-codesign.sh build/cluvio-agent
	scripts/apple-notarize.sh build/cluvio-agent
	tar caf dist/agent-$(AGENT_VERSION)-aarch64-apple-darwin.tar.xz -C build/ cluvio-agent

build-agent-x86_64-pc-windows-msvc: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-pc-windows-msvc \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	strip build/bin/agent.exe
	mv build/bin/agent.exe build/cluvio-agent.exe
	(cd build && \
		7z.exe a -ttar ../dist/agent-$(AGENT_VERSION)-x86_64-pc-windows-msvc.tar -so cluvio-agent.exe | \
		7z.exe a ../dist/agent-$(AGENT_VERSION)-x86_64-pc-windows-msvc.tar.xz -si)

build-setup-x86_64-unknown-linux-musl: export TARGET_CC = cc
build-setup-x86_64-unknown-linux-musl: export TARGET_AR = ar
build-setup-x86_64-unknown-linux-musl: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	strip build/bin/setup
	mv build/bin/setup dist/cluvio-setup

build-setup-aarch64-unknown-linux-musl: export TARGET_CC = aarch64-linux-gnu-gcc
build-setup-aarch64-unknown-linux-musl: export TARGET_AR = aarch64-linux-gnu-ar
build-setup-aarch64-unknown-linux-musl: clean
	mkdir -p build dist
	cargo install \
		--target aarch64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	aarch64-linux-gnu-strip build/bin/setup
	mv build/bin/setup dist/cluvio-setup

# This export can be removed after https://github.com/alexcrichton/xz2-rs/pull/85.
build-setup-x86_64-apple-darwin: export LZMA_API_STATIC = 1
build-setup-x86_64-apple-darwin: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	strip build/bin/setup
	mv build/bin/setup dist/cluvio-setup
	scripts/apple-codesign.sh dist/cluvio-setup
	scripts/apple-notarize.sh dist/cluvio-setup

# This export can be removed after https://github.com/alexcrichton/xz2-rs/pull/85.
build-setup-aarch64-apple-darwin: export LZMA_API_STATIC = 1
build-setup-aarch64-apple-darwin: export SDKROOT = $(shell xcrun -sdk macosx11.1 --show-sdk-path)
build-setup-aarch64-apple-darwin: export MACOSX_DEPLOYMENT_TARGET = $(shell xcrun -sdk macosx11.1 --show-sdk-platform-version)
build-setup-aarch64-apple-darwin: clean
	mkdir -p build dist
	cargo install \
		--target aarch64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	strip build/bin/setup
	mv build/bin/setup dist/cluvio-setup
	scripts/apple-codesign.sh dist/cluvio-setup
	scripts/apple-notarize.sh dist/cluvio-setup

# This export can be removed after https://github.com/alexcrichton/xz2-rs/pull/85.
build-setup-x86_64-pc-windows-msvc: export LZMA_API_STATIC = 1
build-setup-x86_64-pc-windows-msvc: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-pc-windows-msvc \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	strip build/bin/setup.exe
	mv build/bin/setup.exe dist/cluvio-setup.exe

clean:
	rm -rf build/ dist/

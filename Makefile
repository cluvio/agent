AGENT_VERSION = 0.1.0

.PHONY: \
    build-agent-aarch64-unknown-linux-musl \
	build-agent-x86_64-unknown-linux-musl \
	build-agent-x86_64-apple-darwin \
	build-agent-aarch64-apple-darwin \
	build-agent-x86_64-pc-windows-msvc \
	deb-agent-x86_64-unknown-linux-musl \
	deb-agent-aarch64-unknown-linux-musl \
	agent-version

.EXPORT_ALL_VARIABLES:

agent-version:
	@echo $(AGENT_VERSION)

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
	strip build/bin/cluvio-agent
	mv build/bin/cluvio-agent build/cluvio-agent
	tar caf dist/cluvio-agent-$(AGENT_VERSION)-x86_64-unknown-linux-musl.tar.xz -C build/ cluvio-agent

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
	aarch64-linux-gnu-strip build/bin/cluvio-agent
	mv build/bin/cluvio-agent build/cluvio-agent
	tar caf dist/cluvio-agent-$(AGENT_VERSION)-aarch64-unknown-linux-musl.tar.xz -C build/ cluvio-agent

build-agent-x86_64-apple-darwin: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	strip build/bin/cluvio-agent
	mv build/bin/cluvio-agent build/cluvio-agent
	scripts/macos/apple-codesign.sh build/cluvio-agent cluvio-agent-$(AGENT_VERSION)-x86_64-apple-darwin
	scripts/macos/apple-notarize.sh build/cluvio-agent cluvio-agent-$(AGENT_VERSION)-x86_64-apple-darwin
	tar caf dist/cluvio-agent-$(AGENT_VERSION)-x86_64-apple-darwin.tar.xz -C build/ cluvio-agent

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
	strip build/bin/cluvio-agent
	mv build/bin/cluvio-agent build/cluvio-agent
	scripts/macos/apple-codesign.sh build/cluvio-agent cluvio-agent-$(AGENT_VERSION)-aarch64-apple-darwin
	scripts/macos/apple-notarize.sh build/cluvio-agent cluvio-agent-$(AGENT_VERSION)-aarch64-apple-darwin
	tar caf dist/cluvio-agent-$(AGENT_VERSION)-aarch64-apple-darwin.tar.xz -C build/ cluvio-agent

build-agent-x86_64-pc-windows-msvc: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-pc-windows-msvc \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	strip build/bin/cluvio-agent.exe
	mv build/bin/cluvio-agent.exe build/cluvio-agent.exe
	(cd build && 7z.exe a ../dist/cluvio-agent-$(AGENT_VERSION)-x86_64-pc-windows-msvc.zip cluvio-agent.exe)

deb-agent-x86_64-unknown-linux-musl: build-agent-x86_64-unknown-linux-musl
	cargo deb -p cluvio-agent --target=x86_64-unknown-linux-musl

deb-agent-aarch64-unknown-linux-musl: build-agent-aarch64-unknown-linux-musl
	cargo deb -p cluvio-agent --target=aarch64-unknown-linux-musl

rpm-agent-x86_64-unknown-linux-musl: build-agent-x86_64-unknown-linux-musl
	cargo generate-rpm -p agent --target=x86_64-unknown-linux-musl

rpm-agent-aarch64-unknown-linux-musl: build-agent-aarch64-unknown-linux-musl
	cargo generate-rpm -p agent --target=aarch64-unknown-linux-musl

clean:
	rm -rf build/ dist/

AGENT_VERSION = 0.1.0

.PHONY: \
    build-agent-aarch64-unknown-linux-musl \
	build-agent-x86_64-unknown-linux-musl \
	build-agent-x86_64-apple-darwin \
	build-agent-aarch64-apple-darwin \
	build-agent-x86_64-pc-windows-msvc \
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
	scripts/apple-codesign.sh build/cluvio-agent agent-$(AGENT_VERSION)-x86_64-apple-darwin
	scripts/apple-notarize.sh build/cluvio-agent agent-$(AGENT_VERSION)-x86_64-apple-darwin
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
	scripts/apple-codesign.sh build/cluvio-agent agent-$(AGENT_VERSION)-aarch64-apple-darwin
	scripts/apple-notarize.sh build/cluvio-agent agent-$(AGENT_VERSION)-aarch64-apple-darwin
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
	(cd build && zip a ../dist/agent-$(AGENT_VERSION)-x86_64-pc-windows-msvc.zip cluvio-agent.exe)

clean:
	rm -rf build/ dist/

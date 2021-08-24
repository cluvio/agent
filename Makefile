VERSION = 0.1.0

.PHONY: build-aarch64-unknown-linux-musl \
	build-x86_64-unknown-linux-musl \
	build-x86_64-apple-darwin \
	build-aarch64-apple-darwin \
	build-x86_64-pc-windows-msvc \
	version

version:
	@echo $(VERSION)

build-x86_64-unknown-linux-musl: export TARGET_CC = cc
build-x86_64-unknown-linux-musl: export TARGET_AR = ar
build-x86_64-unknown-linux-musl: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	cargo install \
		--target x86_64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	strip build/bin/agent
	strip build/bin/setup
	mv build/bin/agent build/cluvio-agent
	mv build/bin/setup dist/cluvio-setup
	tar caf dist/agent-$(VERSION)-x86_64-unknown-linux-musl.tar.xz -C build/ cluvio-agent

build-aarch64-unknown-linux-musl: export TARGET_CC = aarch64-linux-gnu-gcc
build-aarch64-unknown-linux-musl: export TARGET_AR = aarch64-linux-gnu-ar
build-aarch64-unknown-linux-musl: clean
	mkdir -p build dist
	cargo install \
		--target aarch64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	cargo install \
		--target aarch64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	aarch64-linux-gnu-strip build/bin/agent
	aarch64-linux-gnu-strip build/bin/setup
	mv build/bin/agent build/cluvio-agent
	mv build/bin/setup dist/cluvio-setup
	tar caf dist/agent-$(VERSION)-aarch64-unknown-linux-musl.tar.xz -C build/ cluvio-agent

build-x86_64-apple-darwin: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	cargo install \
		--target x86_64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	strip build/bin/agent
	strip build/bin/setup
	mv build/bin/agent build/cluvio-agent
	mv build/bin/setup dist/cluvio-setup
	tar caf dist/agent-$(VERSION)-x86_64-apple-darwin.tar.xz -C build/ cluvio-agent

build-aarch64-apple-darwin: export SDKROOT = $(shell xcrun -sdk macosx11.1 --show-sdk-path)
build-aarch64-apple-darwin: export MACOSX_DEPLOYMENT_TARGET = $(shell xcrun -sdk macosx11.1 --show-sdk-platform-version)
build-aarch64-apple-darwin: clean
	mkdir -p build dist
	cargo install \
		--target aarch64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	cargo install \
		--target aarch64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	strip build/bin/agent
	strip build/bin/setup
	mv build/bin/agent build/cluvio-agent
	mv build/bin/setup dist/cluvio-setup
	tar caf dist/agent-$(VERSION)-aarch64-apple-darwin.tar.xz -C build/ cluvio-agent

build-x86_64-pc-windows-msvc: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-pc-windows-msvc \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	cargo install \
		--target x86_64-pc-windows-msvc \
		--no-track \
		--locked \
		--root build/ \
		--path setup
	strip build/bin/agent.exe
	strip build/bin/setup.exe
	mv build/bin/agent.exe build/cluvio-agent.exe
	mv build/bin/setup.exe dist/cluvio-setup.exe
	(cd build && \
		7z.exe a -ttar ../dist/agent-$(VERSION)-x86_64-pc-windows-msvc.tar -so cluvio-agent.exe | \
		7z.exe a ../dist/agent-$(VERSION)-x86_64-pc-windows-msvc.tar.xz -si)

clean:
	rm -rf build/ dist/

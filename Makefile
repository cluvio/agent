VERSION = 0.1.0

.PHONY: build-aarch64-unknown-linux-musl build-x86_64-unknown-linux-musl build-apple-darwin clean version

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
	strip build/bin/agent
	mv build/bin/agent build/cluvio-agent
	tar caf dist/agent-eu-$(VERSION)-x86_64-unknown-linux-musl.tar.xz -C build/ cluvio-agent

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
	aarch64-linux-gnu-strip build/bin/agent
	mv build/bin/agent build/cluvio-agent
	tar caf dist/agent-eu-$(VERSION)-aarch64-unknown-linux-musl.tar.xz -C build/ cluvio-agent

build-x86_64-apple-darwin: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	strip build/bin/agent
	mv build/bin/agent build/cluvio-agent
	tar caf dist/agent-eu-$(VERSION)-x86_64-apple-darwin.tar.gz -C build/ cluvio-agent

clean:
	rm -rf build/ dist/

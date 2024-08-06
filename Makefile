AGENT_VERSION := $(shell cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "cluvio-agent") | .version')

.PHONY: \
    build-agent-aarch64-linux \
	build-agent-x86_64-linux \
	build-agent-x86_64-macos \
	build-agent-aarch64-macos \
	build-agent-x86_64-windows \
	deb-agent-x86_64 \
	deb-agent-aarch64 \
    rpm-agent-x86_64 \
    rpm-agent-aarch64 \
	agent-version

.EXPORT_ALL_VARIABLES:

agent-version:
	@echo $(AGENT_VERSION)

build-agent-x86_64-linux: clean
	mkdir -p build dist
	cross build --release --target x86_64-unknown-linux-musl --locked
	cp target/x86_64-unknown-linux-musl/release/cluvio-agent build/cluvio-agent
	tar caf dist/cluvio-agent-$(AGENT_VERSION)-x86_64-linux.tar.xz -C build/ cluvio-agent

build-agent-aarch64-linux: clean
	mkdir -p build dist
	cross build --release --target aarch64-unknown-linux-musl --locked
	cp target/aarch64-unknown-linux-musl/release/cluvio-agent build/cluvio-agent
	tar caf dist/cluvio-agent-$(AGENT_VERSION)-aarch64-linux.tar.xz -C build/ cluvio-agent

build-agent-x86_64-macos: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	mv build/bin/cluvio-agent build/cluvio-agent
	scripts/macos/apple-codesign.sh build/cluvio-agent cluvio-agent-$(AGENT_VERSION)-x86_64-macos
	scripts/macos/apple-notarize.sh build/cluvio-agent cluvio-agent-$(AGENT_VERSION)-x86_64-macos
	tar caf dist/cluvio-agent-$(AGENT_VERSION)-x86_64-macos.tar.xz -C build/ cluvio-agent

build-agent-aarch64-macos: export SDKROOT = $(shell xcrun --show-sdk-path)
build-agent-aarch64-macos: export MACOSX_DEPLOYMENT_TARGET = $(shell xcrun --show-sdk-platform-version)
build-agent-aarch64-macos: clean
	@echo "SDKROOT: $(SDKROOT)"
	@echo "MACOSX_DEPLOYMENT_TARGET: $(MACOSX_DEPLOYMENT_TARGET)"
	mkdir -p build dist
	cargo install \
		--target aarch64-apple-darwin \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	mv build/bin/cluvio-agent build/cluvio-agent
	scripts/macos/apple-codesign.sh build/cluvio-agent cluvio-agent-$(AGENT_VERSION)-aarch64-macos
	scripts/macos/apple-notarize.sh build/cluvio-agent cluvio-agent-$(AGENT_VERSION)-aarch64-macos
	tar caf dist/cluvio-agent-$(AGENT_VERSION)-aarch64-macos.tar.xz -C build/ cluvio-agent

build-agent-x86_64-windows: export RUSTFLAGS = -C target-feature=+crt-static
build-agent-x86_64-windows: clean
	mkdir -p build dist
	cargo install \
		--target x86_64-pc-windows-msvc \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	mv build/bin/cluvio-agent.exe build/cluvio-agent.exe
	(cd build && 7z.exe a ../dist/cluvio-agent-$(AGENT_VERSION)-x86_64-windows.zip cluvio-agent.exe)

docker-agent-x86_64-linux: build-agent-x86_64-linux
	test -n "$(DOCKER_HUB_USERNAME)" # $$DOCKER_HUB_USERNAME
	test -n "$(DOCKER_HUB_ACCESS_TOKEN)" # $$DOCKER_HUB_ACCESS_TOKEN
	@echo "${DOCKER_HUB_ACCESS_TOKEN}" | docker login --username $(DOCKER_HUB_USERNAME) --password-stdin
	docker build -t cluvio/agent:$(AGENT_VERSION)-x86_64 .
	docker push --all-tags cluvio/agent

docker-agent-aarch64-linux: build-agent-aarch64-linux
	test -n "$(DOCKER_HUB_USERNAME)" # $$DOCKER_HUB_USERNAME
	test -n "$(DOCKER_HUB_ACCESS_TOKEN)" # $$DOCKER_HUB_ACCESS_TOKEN
	@echo "${DOCKER_HUB_ACCESS_TOKEN}" | docker login --username $(DOCKER_HUB_USERNAME) --password-stdin
	docker build --build-arg ARCH=arm64v8/ -t cluvio/agent:$(AGENT_VERSION)-arm64 .
	docker push --all-tags cluvio/agent

docker-agent-release:
	test -n "$(DOCKER_HUB_USERNAME)" # $$DOCKER_HUB_USERNAME
	test -n "$(DOCKER_HUB_ACCESS_TOKEN)" # $$DOCKER_HUB_ACCESS_TOKEN
	@echo "${DOCKER_HUB_ACCESS_TOKEN}" | docker login --username $(DOCKER_HUB_USERNAME) --password-stdin
	docker manifest create cluvio/agent:$(AGENT_VERSION) \
		--amend cluvio/agent:$(AGENT_VERSION)-arm64 \
		--amend cluvio/agent:$(AGENT_VERSION)-x86_64
	docker manifest annotate cluvio/agent:$(AGENT_VERSION) cluvio/agent:$(AGENT_VERSION)-x86_64 --arch amd64
	docker manifest annotate cluvio/agent:$(AGENT_VERSION) cluvio/agent:$(AGENT_VERSION)-arm64 --arch arm64 --variant v8
	docker manifest push cluvio/agent:$(AGENT_VERSION)
	docker manifest create cluvio/agent:latest \
		--amend cluvio/agent:$(AGENT_VERSION)-arm64 \
		--amend cluvio/agent:$(AGENT_VERSION)-x86_64
	docker manifest annotate cluvio/agent:latest cluvio/agent:$(AGENT_VERSION)-x86_64 --arch amd64
	docker manifest annotate cluvio/agent:latest cluvio/agent:$(AGENT_VERSION)-arm64 --arch arm64 --variant v8
	docker manifest push cluvio/agent:latest

deb-agent-x86_64: build-agent-x86_64-linux
	cargo deb -p cluvio-agent --target=x86_64-unknown-linux-musl --no-build --no-strip

deb-agent-aarch64: build-agent-aarch64-linux
	cargo deb -p cluvio-agent --target=aarch64-unknown-linux-musl --no-build --no-strip

rpm-agent-x86_64: build-agent-x86_64-linux
	cargo generate-rpm -p agent --target=x86_64-unknown-linux-musl

rpm-agent-aarch64: build-agent-aarch64-linux
	cargo generate-rpm -p agent --target=aarch64-unknown-linux-musl

clean:
	rm -rf build/ dist/

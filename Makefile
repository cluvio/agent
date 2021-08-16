VERSION = 0.1.0

.PHONY: build-linux-musl build-apple-darwin clean version

version:
	echo $(VERSION)

build-linux-musl:
	mkdir -p build dist
	cargo install \
		--target x86_64-unknown-linux-musl \
		--no-track \
		--locked \
		--root build/ \
		--path agent
	strip build/bin/agent
	mv build/bin/agent build/cluvio-agent
	tar caf dist/agent-eu-$(VERSION)-x86_64-linux.tar.xz -C build/ cluvio-agent

build-apple-darwin:
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

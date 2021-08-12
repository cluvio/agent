VERSION = 0.1.0

.PHONY: build-linux-musl clean

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
	cd dist && sha256sum *.tar.xz >> CHECKSUMS

clean:
	rm -rf build/ dist/

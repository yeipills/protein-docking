.PHONY: help build web server dev test check fmt sample docker oracle

help:
	@echo "make build    Build the web client and the release binary"
	@echo "make dev      Run the server (port 8080) and the Vite dev server (port 5173)"
	@echo "make test     Run Rust and web tests"
	@echo "make check    Formatting, lints and type checks"
	@echo "make sample   Compute the bundled sample in both modes into ./out"
	@echo "make docker   Build the Docker image"
	@echo "make oracle   Re-run the original Python scripts and compare (needs Python 3)"

build: web server

web:
	cd web && npm ci && npm run build

server:
	cargo build --release -p cshape

dev:
	cargo build --release -p cshape
	./target/release/cshape serve & cd web && npm run dev

test:
	cargo test --workspace
	cd web && npm test

check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cd web && npm run typecheck

fmt:
	cargo fmt --all

sample: server
	./target/release/cshape run --vert data/samples/1AHW_l_u/1AHW_l_u.vert \
		--face data/samples/1AHW_l_u/1AHW_l_u.face --out out/corrected
	./target/release/cshape run --mode legacy --vert data/samples/1AHW_l_u/1AHW_l_u.vert \
		--face data/samples/1AHW_l_u/1AHW_l_u.face --stl data/samples/1AHW_l_u/1AHW_l_u.stl \
		--intermediate --out out/legacy

docker:
	docker build -t cshape:3 .

oracle: sample
	tools/legacy-oracle/run.sh out/oracle
	./target/release/cshape compare out/oracle out/legacy

.PHONY: help build web server dev test check fmt sample dock benchmark-data bench docker oracle

help:
	@echo "make build    Build the web client and the release binary"
	@echo "make dev      Run the server (port 8080) and the Vite dev server (port 5173)"
	@echo "make test     Run Rust and web tests"
	@echo "make check    Formatting, lints and type checks"
	@echo "make sample   Compute the bundled sample in both modes into ./out"
	@echo "make dock     Dock the bundled 1AHW complex and score it against the crystal structure"
	@echo "make bench    Docking Benchmark 5.5, rigid category (downloads it first; ~25 min)"
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

dock: server
	./target/release/cshape dock --receptor data/complexes/1AHW/1AHW_r_u.pdb \
		--ligand data/complexes/1AHW/1AHW_l_u.pdb \
		--native-receptor data/complexes/1AHW/1AHW_r_b.pdb \
		--native-ligand data/complexes/1AHW/1AHW_l_b.pdb \
		--randomize-seed 10142002940977943103 --out out/dock-1AHW

benchmark-data:
	test -d data/benchmark/benchmark5.5 || tools/benchmark/fetch.sh

bench: server benchmark-data
	./target/release/cshape bench --category rigid --out out/bench-rigid

docker:
	docker build -t cshape:3 .

oracle: sample
	tools/legacy-oracle/run.sh out/oracle
	./target/release/cshape compare out/oracle out/legacy

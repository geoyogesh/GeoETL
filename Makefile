.PHONY: fmt lint test security coverage coverage-open check help

help:
	@echo "Available targets:"
	@echo "  make fmt            # Format Rust code and TOML files"
	@echo "  make lint           # Run Clippy with pedantic warnings denied"
	@echo "  make test           # Run all workspace tests"
	@echo "  make security       # Run cargo audit and cargo deny"
	@echo "  make coverage       # Generate coverage summary (text)"
	@echo "  make coverage-open  # Generate coverage report and open in browser"
	@echo "  make check          # Run fmt, lint, test, security, and coverage"

fmt:
	cargo fmt --all
	taplo format

lint:
	cargo clippy --workspace --all-targets -- -D warnings -D clippy::pedantic

test:
	cargo test --workspace --all-targets

security:
	cargo audit
	cargo deny check

coverage:
	cargo llvm-cov --workspace --all-targets --fail-under-lines 80

coverage-open:
	cargo llvm-cov --workspace --all-targets --open --fail-under-lines 80

check: fmt lint test security coverage

.PHONY: install build run stop restart test logs

BINARY=target/release/homepage-back-end
LOG_FILE=server.log

install:
	cargo fetch

build:
	cargo build --release

run: build
	@setsid ./$(BINARY) > $(LOG_FILE) 2>&1 &
	@echo "Back rodando, log em $(LOG_FILE)"

stop:
	@echo "Stopping..."
	@-pkill -SIGTERM -f "^\./$(BINARY)"
	@echo "Stopped!"

restart: stop run

test:
	cargo test
	cargo clippy --all-targets -- -D warnings

logs:
	@trap 'exit 0' INT; tail -n 50 -f $(LOG_FILE)

PREFIX  ?= /usr
BINDIR  ?= $(PREFIX)/bin
DESTDIR ?=

BINARY     := linkdrop
RELEASE_BIN := target/release/$(BINARY)

.PHONY: build install uninstall clean

# Build the release binary (run as your normal user, not sudo)
build:
	cargo build --release

# Install the already-built CLI to BINDIR (default /usr/bin).
# Run `make build` first as your user; then `sudo make install` to copy.
# Only the copy needs root — building under sudo breaks rustup.
install:
	@test -f $(RELEASE_BIN) || { \
		echo "error: $(RELEASE_BIN) not found — run 'make build' first (as your user, not sudo)"; \
		exit 1; \
	}
	install -d $(DESTDIR)$(BINDIR)
	install -m 0755 $(RELEASE_BIN) $(DESTDIR)$(BINDIR)/$(BINARY)
	@echo "installed $(BINARY) -> $(DESTDIR)$(BINDIR)/$(BINARY)"

# Remove the CLI from BINDIR
uninstall:
	rm -f $(DESTDIR)$(BINDIR)/$(BINARY)
	@echo "removed $(BINARY) from $(DESTDIR)$(BINDIR)"

clean:
	cargo clean

SHELL := /bin/bash
REPO_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
OUTDIR := $(if $(strip $(outdir)),$(abspath $(outdir)),$(REPO_DIR))
SPEC := $(if $(strip $(spec)),$(abspath $(spec)),$(REPO_DIR)/lsi.spec)

.PHONY: srpm
srpm:
	@set -euo pipefail; \
	if ! command -v cargo >/dev/null 2>&1 || ! command -v rpmbuild >/dev/null 2>&1 || ! command -v xz >/dev/null 2>&1 || ! command -v git >/dev/null 2>&1; then \
		if command -v dnf >/dev/null 2>&1; then \
			dnf -y install cargo rust rpm-build tar xz git-core; \
		else \
			echo "Missing required tools (cargo, rpmbuild, tar, xz, git)" >&2; \
			exit 1; \
		fi; \
	fi; \
	WORKDIR="$$(mktemp -d)"; \
	trap 'rm -rf "$$WORKDIR"' EXIT; \
	VERSION="$$(awk -F'"' '/^version[[:space:]]*=/ {print $$2; exit}' "$(REPO_DIR)/Cargo.toml")"; \
	if [ -z "$$VERSION" ]; then \
		echo "Failed to read package version from $(REPO_DIR)/Cargo.toml" >&2; \
		exit 1; \
	fi; \
	RPM_VERSION="$$(printf '%s' "$$VERSION" | tr '-' '~')"; \
	git -C "$(REPO_DIR)" archive --format=tar.gz --prefix="lsi-$${RPM_VERSION}/" HEAD -o "$$WORKDIR/lsi-$${RPM_VERSION}.tar.gz"; \
	cargo vendor --manifest-path "$(REPO_DIR)/Cargo.toml" --locked "$$WORKDIR/vendor" >/dev/null; \
	tar -C "$$WORKDIR" -cJf "$$WORKDIR/lsi-$${RPM_VERSION}-vendor.tar.xz" vendor; \
	sed -E "s/^Version:[[:space:]]*.*/Version:        $${RPM_VERSION}/" "$(SPEC)" > "$$WORKDIR/lsi.spec"; \
	mkdir -p "$(OUTDIR)"; \
	rpmbuild -bs \
		--define "_sourcedir $$WORKDIR" \
		--define "_srcrpmdir $(OUTDIR)" \
		"$$WORKDIR/lsi.spec"

%global debug_package %{nil}

Name:           lsi
Version:        1.4.0
Release:        1%{?dist}
Summary:        An interactive eza-like terminal file explorer with Nerd Fonts and themes

License:        MIT
URL:            https://github.com/aissasell/lsi
Source0:        %{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.xz

BuildRequires:  cargo
BuildRequires:  rust >= 1.88
BuildRequires:  gcc

%description
lsi is an interactive eza-inspired terminal file explorer written in Rust.
It renders inline directly below your shell prompt with Nerd Font icons,
metadata columns, file operations, and a live preview pane.

%prep
%autosetup -n %{name}-%{version} -a 1

mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%build
cargo build --release --locked --offline

%install
install -Dpm 0755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

%files
%license LICENSE
%doc README.md
%{_bindir}/%{name}

%changelog
* Thu Nov 26 2026 Salem Ben Aissa <aissasell@gmail.com> - 1.4.0-1
- Initial Fedora Copr package

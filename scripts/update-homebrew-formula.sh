#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: $0 <version> <sha256sums-file> [output-formula-path]"
  echo "Example: $0 0.1.6 ./SHA256SUMS"
  echo "Example: $0 0.1.6 ./SHA256SUMS ./Formula/hurl-lsp.rb"
  exit 1
fi

version="$1"
sums_file="$2"
formula_path="${3:-packaging/homebrew/Formula/hurl-lsp.rb}"

if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z]+)*$ ]]; then
  echo "Invalid version: ${version}"
  echo "Expected semver-like value, for example: 0.1.6 or 0.1.6-rc1"
  exit 1
fi

if [[ ! -f "${sums_file}" ]]; then
  echo "Missing checksum file: ${sums_file}"
  exit 1
fi

extract_sha() {
  local target="$1"
  awk -v file="${target}" '$2 == file { print $1 }' "${sums_file}"
}

darwin_arm_file="hurl-lsp-${version}-aarch64-apple-darwin.tar.gz"
darwin_x64_file="hurl-lsp-${version}-x86_64-apple-darwin.tar.gz"
linux_x64_file="hurl-lsp-${version}-x86_64-unknown-linux-gnu.tar.gz"

darwin_arm_sha="$(extract_sha "${darwin_arm_file}")"
darwin_x64_sha="$(extract_sha "${darwin_x64_file}")"
linux_x64_sha="$(extract_sha "${linux_x64_file}")"

if [[ -z "${darwin_arm_sha}" || -z "${darwin_x64_sha}" || -z "${linux_x64_sha}" ]]; then
  echo "Could not find all required checksums in ${sums_file}"
  exit 1
fi

cat > "${formula_path}" <<EOF
class HurlLsp < Formula
  desc "Language Server Protocol implementation for Hurl"
  homepage "https://github.com/testmind-hq/hurl-lsp"
  license "MIT"
  version "${version}"

  on_macos do
    on_arm do
      url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "${darwin_arm_sha}"
    end
    on_intel do
      url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "${darwin_x64_sha}"
    end
  end

  on_linux do
    url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "${linux_x64_sha}"
  end

  def install
    bin.install "hurl-lsp"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hurl-lsp --version")
  end
end
EOF

echo "Updated ${formula_path} to v${version}"

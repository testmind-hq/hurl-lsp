class HurlLsp < Formula
  desc "Language Server Protocol implementation for Hurl"
  homepage "https://github.com/testmind-hq/hurl-lsp"
  license "MIT"
  version "0.1.9"

  on_macos do
    on_arm do
      url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "cb67668542216cc4683473390ee59a3445b6916e751e6e12381c0734f002487b"
    end
    on_intel do
      url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "6d6d2902e57c6025d753c4e3dbb9ecfd7d99acceeb0baf876078dce67a3f7f42"
    end
  end

  on_linux do
    url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "b4ae7462d202d2199c5f48b44f757dd509ed0761bb0d1e8ce989f22d5573f3d9"
  end

  def install
    bin.install "hurl-lsp"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hurl-lsp --version")
  end
end

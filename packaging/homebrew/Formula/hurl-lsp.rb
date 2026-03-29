class HurlLsp < Formula
  desc "Language Server Protocol implementation for Hurl"
  homepage "https://github.com/testmind-hq/hurl-lsp"
  license "MIT"
  version "0.1.11"

  on_macos do
    on_arm do
      url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "998fbe8ae2b553c3b2eb589d7eaf47bbed4c0332e446706f07c3a96b4409611e"
    end
    on_intel do
      url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "f2fb8d052785590dc7497c3cc08643340a640ee656569a485a2cf9828e84f6de"
    end
  end

  on_linux do
    url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "6aa99b059860bf8f8e85611dffe21a29ee125bbf4ab1510ccc047d1455a9fca1"
  end

  def install
    bin.install "hurl-lsp"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hurl-lsp --version")
  end
end

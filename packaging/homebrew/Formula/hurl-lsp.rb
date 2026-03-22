class HurlLsp < Formula
  desc "Language Server Protocol implementation for Hurl"
  homepage "https://github.com/testmind-hq/hurl-lsp"
  license "MIT"
  version "0.1.5"

  on_macos do
    on_arm do
      url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "5229b68016fe58ee81896c3556f4b2555b219ba8f951b6e411bde6ff8f7afc5a"
    end
    on_intel do
      url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "cf8caffebe461be600d14a125487fdf2b3c94fc675515bd3524d3a61a4f8ba4b"
    end
  end

  on_linux do
    url "https://github.com/testmind-hq/hurl-lsp/releases/download/v#{version}/hurl-lsp-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "4987cea706e52b52106ec973bb22d07b2a87bad31140e512547ee121dbfc502c"
  end

  def install
    bin.install "hurl-lsp"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hurl-lsp --version")
  end
end

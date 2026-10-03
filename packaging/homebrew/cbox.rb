class Cbox < Formula
  desc "A tiny Rust TUI launcher for saving, fuzzy-finding, and injecting shell commands"
  homepage "https://github.com/mukh4w/cbox"
  version "0.1.0"
  
  if OS.mac? && Hardware::CPU.intel?
    url "https://github.com/mukh4w/cbox/releases/download/v0.1.0/cbox-x86_64-apple-darwin.tar.gz"
    sha256 "784666486fd92b006e0364825951a0d2892dffc47655321283fab6638d46c19f"
  elsif OS.mac? && Hardware::CPU.arm?
    url "https://github.com/mukh4w/cbox/releases/download/v0.1.0/cbox-aarch64-apple-darwin.tar.gz"
    sha256 "832c4ff1aac09d221901b439252bff0c4c6b20d8214780108b4ae41e48cd7fd8"
  elsif OS.linux? && Hardware::CPU.intel?
    url "https://github.com/mukh4w/cbox/releases/download/v0.1.0/cbox-x86_64-unknown-linux-musl.tar.gz"
    sha256 "bcf9b740e55b41f007d8567f236eb51b6f9c6e3188e1845e88f3323f8b13895f"
  elsif OS.linux? && Hardware::CPU.arm?
    url "https://github.com/mukh4w/cbox/releases/download/v0.1.0/cbox-aarch64-unknown-linux-musl.tar.gz"
    sha256 "85811e08b0fdaa2365bd9e8f16c5553bc1cb3d2e661d8d1e97dde8fe6a097df3"
  end

  def install
    bin.install "cbox"
  end

  def caveats
    <<~EOS
      To enable shell integration, add the following to your configuration:
        bash: eval "$(cbox init bash)"
        zsh:  eval "$(cbox init zsh)"
        fish: cbox init fish | source
    EOS
  end
end

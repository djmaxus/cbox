class Cbox < Formula
  desc "A tiny Rust TUI launcher for saving, fuzzy-finding, and injecting shell commands"
  homepage "https://github.com/mukh4w/cbox"
  version "0.1.0"
  
  if OS.mac? && Hardware::CPU.intel?
    url "https://github.com/mukh4w/cbox/releases/download/v0.1.0/cbox-x86_64-apple-darwin.tar.gz"
  elsif OS.mac? && Hardware::CPU.arm?
    url "https://github.com/mukh4w/cbox/releases/download/v0.1.0/cbox-aarch64-apple-darwin.tar.gz"
  elsif OS.linux? && Hardware::CPU.intel?
    url "https://github.com/mukh4w/cbox/releases/download/v0.1.0/cbox-x86_64-unknown-linux-musl.tar.gz"
  elsif OS.linux? && Hardware::CPU.arm?
    url "https://github.com/mukh4w/cbox/releases/download/v0.1.0/cbox-aarch64-unknown-linux-musl.tar.gz"
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

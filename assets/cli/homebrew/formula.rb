class {{CLASS}} < Formula
  desc "CLI tool for managing Patr cloud resources"
  homepage "https://github.com/patr-cloud/patr"
  version "{{VERSION}}"

  conflicts_with "{{CONFLICTS_A}}", because: "both install the patr binary"
  conflicts_with "{{CONFLICTS_B}}", because: "both install the patr binary"

  on_macos do
    if Hardware::CPU.arm?
      url "{{BASE_URL}}/patr-darwin-arm64.zip"
      sha256 "{{SHA_DARWIN_ARM64}}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "{{BASE_URL}}/patr-linux-arm64.tar.gz"
      sha256 "{{SHA_LINUX_ARM64}}"
    else
      url "{{BASE_URL}}/patr-linux-amd64.tar.gz"
      sha256 "{{SHA_LINUX_AMD64}}"
    end
  end

  def install
    bin.install "patr"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/patr --version")
  end
end

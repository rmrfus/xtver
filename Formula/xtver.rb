class Xtver < Formula
  desc "Query terminal XTVERSION and print the result"
  homepage "https://github.com/rmrfus/xtver"
  url "https://github.com/rmrfus/xtver/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # no real tty in test sandbox — expect failure with a known error message
    output = shell_output("#{bin}/xtver 2>&1", 1)
    assert_match "error", output
  end
end

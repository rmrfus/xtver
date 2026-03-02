class Xtver < Formula
  desc "Query terminal XTVERSION and print the result"
  homepage "https://github.com/rmrfus/xtver"
  url "https://github.com/rmrfus/xtver/archive/refs/tags/v0.1.1.tar.gz"
  sha256 "9dcc71b7fe09814db70138dbd712e91245c87c1a7dd5f3314993fe7b07e49ea7"
  license "GPL-3.0-only"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    man1.install "man/man1/xtver.1"
  end

  test do
    # no real tty in test sandbox — expect failure with a known error message
    output = shell_output("#{bin}/xtver 2>&1", 1)
    assert_match "error", output
  end
end

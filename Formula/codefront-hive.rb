class CodefrontHive < Formula
  desc "Terminal multiplexer for AI coding agents"
  homepage "https://hive.codefrontlabs.com/"
  version "0.5.11"
  license "MIT"
  conflicts_with "hive", because: "both install a hive executable"

  on_macos do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.5.11/hive-aarch64-apple-darwin.tar.gz"
      sha256 "9c810ccdac85d22b24a58828ab0c53c4c3d1b63d873d3d03af1038e685a928c7"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.5.11/hive-x86_64-apple-darwin.tar.gz"
      sha256 "05cb9b992b9702ab1d6ec3371bc9cd3a1fadaf7cf5388f9a88882db54079149e"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.5.11/hive-aarch64-unknown-linux-musl.tar.gz"
      sha256 "ab45a480ac2242f4f9ae00ab7d0d2850352caa8e3d028952ba381a8cc33b0438"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.5.11/hive-x86_64-unknown-linux-musl.tar.gz"
      sha256 "5e05671386a01e1e7560aab0bb7ce3544011cf1b3fab4974ca1fbdcb659ed093"
    end
  end

  def install
    bin.install "hive", "hived"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/hive --version")
    assert_predicate bin/"hived", :executable?
  end
end

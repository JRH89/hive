class CodefrontHive < Formula
  desc "Terminal multiplexer for AI coding agents"
  homepage "https://hive.codefrontlabs.com/"
  version "0.6.0"
  license "MIT"
  conflicts_with "hive", because: "both install a hive executable"

  on_macos do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.6.0/hive-aarch64-apple-darwin.tar.gz"
      sha256 "8fd435b0246c3da4f8b142b4624418b3a4d303fac3f4a7112394f5988ff63a8c"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.6.0/hive-x86_64-apple-darwin.tar.gz"
      sha256 "608cd054fab799a6cc2bcbba3b66b37373cd741a6caa44933c387b97de98a33d"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.6.0/hive-aarch64-unknown-linux-musl.tar.gz"
      sha256 "43e994dd3db2845838b4e9ea972d4676273f6ab5b5c70fb679989b9ee5f5844c"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.6.0/hive-x86_64-unknown-linux-musl.tar.gz"
      sha256 "f7b710d2a95ca17ddb3f3f3cceddb4c6b4034372b9484c2f2221d6123b231cf7"
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

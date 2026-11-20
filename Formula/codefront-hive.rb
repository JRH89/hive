class CodefrontHive < Formula
  desc "Terminal multiplexer for AI coding agents"
  homepage "https://hive.codefrontlabs.com/"
  version "0.5.14"
  license "MIT"
  conflicts_with "hive", because: "both install a hive executable"

  on_macos do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.5.14/hive-aarch64-apple-darwin.tar.gz"
      sha256 "eef6af026beed2895e5c5331cbeb7ad65099e671b6825f01ebc51f7268a63f4d"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.5.14/hive-x86_64-apple-darwin.tar.gz"
      sha256 "bb7a2d5732697b437ca01fd31f5afbbfeb83396e879a7a8e9e4958062ebc4ca2"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.5.14/hive-aarch64-unknown-linux-musl.tar.gz"
      sha256 "b344c4590d8aa92a1a1f258e02cf2c9372e682a3d0ec13b58e2a4aeca1d2bce0"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.5.14/hive-x86_64-unknown-linux-musl.tar.gz"
      sha256 "bbc0baf523dde6ba687ae8cee9aa0354c8934fe0413b6c680c19a14708f19dfd"
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

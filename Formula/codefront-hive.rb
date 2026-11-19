class CodefrontHive < Formula
  desc "Terminal multiplexer for AI coding agents"
  homepage "https://hive.codefrontlabs.com/"
  version "0.5.12"
  license "MIT"
  conflicts_with "hive", because: "both install a hive executable"

  on_macos do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.5.12/hive-aarch64-apple-darwin.tar.gz"
      sha256 "9dc8f7d2cbcc9b75e78e6a4bef4007ead8a69358ca1fb6994aadcd21d86d837c"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.5.12/hive-x86_64-apple-darwin.tar.gz"
      sha256 "7a7dd0f04c08cb650ae5ec1a98319e9f3f93e1017d90e763489ac4c6cadafec5"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.5.12/hive-aarch64-unknown-linux-musl.tar.gz"
      sha256 "307ad0fee8b87dc1afe4991901d23f1e842423fa242d812fe759020b59deb071"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.5.12/hive-x86_64-unknown-linux-musl.tar.gz"
      sha256 "c06e2e7ef9e3fceeb9d660c6e767d848181a744ce2d06b9899cc52702697054b"
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

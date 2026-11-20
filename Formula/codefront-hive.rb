class CodefrontHive < Formula
  desc "Terminal multiplexer for AI coding agents"
  homepage "https://hive.codefrontlabs.com/"
  version "0.5.13"
  license "MIT"
  conflicts_with "hive", because: "both install a hive executable"

  on_macos do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.5.13/hive-aarch64-apple-darwin.tar.gz"
      sha256 "c50fb4f62ec1180285a1f099aec89f05c9936c977d1d8d518e75941db60e3e6a"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.5.13/hive-x86_64-apple-darwin.tar.gz"
      sha256 "60ec64ca3c437dff954b372b46fd8be6bcee265484be67a752d7b06268aa8095"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.5.13/hive-aarch64-unknown-linux-musl.tar.gz"
      sha256 "4cd34ac18eeefb3be75d72e1e7967693682371c6935f2f63f97d78b85fd70896"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.5.13/hive-x86_64-unknown-linux-musl.tar.gz"
      sha256 "7f27b41c23e040c53ec8e16f05b54b93d51bc05ba05847b664761284eb740c1b"
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

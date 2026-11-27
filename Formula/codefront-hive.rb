class CodefrontHive < Formula
  desc "Terminal multiplexer for AI coding agents"
  homepage "https://hive.codefrontlabs.com/"
  version "0.6.1"
  license "MIT"
  conflicts_with "hive", because: "both install a hive executable"

  on_macos do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.6.1/hive-aarch64-apple-darwin.tar.gz"
      sha256 "a8b2973a465274b76ef3f02253152f17764ba23406e640c9f6b3e2066d44ff1e"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.6.1/hive-x86_64-apple-darwin.tar.gz"
      sha256 "dad89d8696a714a765286a14a57045f8494a6f92a2aa73799380211723da91ef"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/JRH89/hive/releases/download/v0.6.1/hive-aarch64-unknown-linux-musl.tar.gz"
      sha256 "ead2ed25db46bef2eb16fbd4866fe312e19737e0848a0170a0ec6f6325ffd33d"
    end
    on_intel do
      url "https://github.com/JRH89/hive/releases/download/v0.6.1/hive-x86_64-unknown-linux-musl.tar.gz"
      sha256 "ae2f18befb9bc502ec1458af2ff9028676586bbf7a3c7dace971b9ebfbe7866e"
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

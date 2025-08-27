#!/data/data/com.termux/files/usr/bin/bash
set -e

# Pastikan rust sudah ada
if ! command -v cargo &>/dev/null; then
  echo "Rust belum terinstall. Install dulu dengan:"
  echo "pkg install rust"
  exit 1
fi

# Build release
echo "🚀 Compiling project in release mode..."
cargo build --release

# Binary hasil compile
BINARY_NAME="nama_program_kamu"
OUTPUT="./target/release/$BINARY_NAME"

if [ -f "$OUTPUT" ]; then
  echo "✅ Build sukses! Binary ada di: $OUTPUT"
else
  echo "❌ Build gagal!"
  exit 1
fi

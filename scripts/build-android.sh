#!/usr/bin/env bash
# Builds an Android APK (RG477V, AYN Odin 2 on stock Android, phones with a pad) into dist/:
# the game as libmain.so for arm64 (devices) and x86_64 (the emulator) with cargo-ndk, SDL's Java
# around it (android/), signed with a local sideload key. One-time setup: scripts/setup-android.sh.
set -e
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export ANDROID_HOME=/opt/android-sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/26.3.11579264
export JAVA_HOME=$(dirname $(dirname $(readlink -f $(which javac))))
export CARGO_TARGET_DIR="$HOME/.cache/ashensanctum-android-target"
ABIS="${ABIS:-arm64-v8a x86_64}"
JNI=android/app/src/main/jniLibs
rm -rf "$JNI"
T=""
for a in $ABIS; do T="$T -t $a"; done
cargo ndk $T -P 24 -o "$JNI" build --release --manifest-path android/rust/Cargo.toml
# SDL and hidapi were built as shared libraries next to the game: ship them too.
for a in $ABIS; do
  case $a in
    arm64-v8a) triple=aarch64-linux-android ;;
    x86_64) triple=x86_64-linux-android ;;
  esac
  for lib in libSDL2.so libhidapi.so; do
    f=$(find "$CARGO_TARGET_DIR/$triple/release" -name "$lib" | head -1)
    [ -n "$f" ] && cp "$f" "$JNI/$a/"
  done
  ls -la "$JNI/$a"
done
# The sideload signing key (made once, kept out of git).
if [ ! -f android/sideload.keystore ]; then
  keytool -genkeypair -v -keystore android/sideload.keystore -alias ashensanctum -keyalg RSA -keysize 2048 \
    -validity 10000 -storepass ashensanctum -keypass ashensanctum -dname "CN=Ashen Sanctum, O=GrayHatLabs" > /dev/null
fi
echo "sdk.dir=$ANDROID_HOME" > android/local.properties
(cd android && chmod +x gradlew && ./gradlew -q assembleRelease)
mkdir -p dist
cp android/app/build/outputs/apk/release/app-release.apk dist/AshenSanctum.apk
ls -la dist/AshenSanctum.apk
echo "Built dist/AshenSanctum.apk"

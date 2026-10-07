#!/usr/bin/env bash
# One-time setup in WSL for Android builds (scripts/build-android.sh): JDK 17, the Android SDK + NDK,
# an emulator image for testing, Rust's Android targets and cargo-ndk. Accepts the Android SDK licences.
set -e
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq openjdk-17-jdk-headless unzip curl > /dev/null
SDK=/opt/android-sdk
mkdir -p "$SDK/cmdline-tools"
if [ ! -x "$SDK/cmdline-tools/latest/bin/sdkmanager" ]; then
  curl -sSL -o /tmp/cmdtools.zip https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
  rm -rf /tmp/cmdtools && unzip -q /tmp/cmdtools.zip -d /tmp/cmdtools
  rm -rf "$SDK/cmdline-tools/latest" && mv /tmp/cmdtools/cmdline-tools "$SDK/cmdline-tools/latest"
fi
export ANDROID_HOME=$SDK
SM="$SDK/cmdline-tools/latest/bin/sdkmanager"
yes | "$SM" --licenses > /dev/null
"$SM" --install "platform-tools" "platforms;android-34" "build-tools;34.0.0" "ndk;26.3.11579264" > /dev/null
"$SM" --install "emulator" "system-images;android-34;google_apis;x86_64" > /dev/null
/root/.cargo/bin/rustup target add aarch64-linux-android x86_64-linux-android
/root/.cargo/bin/cargo install cargo-ndk --locked -q
echo "android setup done"
ls "$SDK" "$SDK/ndk"
ls -la /dev/kvm 2>&1 || true

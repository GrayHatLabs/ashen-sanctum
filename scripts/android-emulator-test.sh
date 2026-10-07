#!/usr/bin/env bash
# Boots a headless Android 14 emulator (x86_64), installs dist/AshenSanctum.apk, starts it and saves a
# screenshot to dist/android_test.png (and the app's log to dist/android_log.txt).
set -e
cd "$(dirname "$0")/.."
export ANDROID_HOME=/opt/android-sdk
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
if ! avdmanager list avd | grep -q ashen_test; then
  echo no | avdmanager create avd -n ashen_test -k "system-images;android-34;google_apis;x86_64" -d pixel_6 > /dev/null
fi
# The screen: SCREEN=1280x960 (RG477V, 4:3; the default) or SCREEN=1920x1080 (AYN Odin 2, 16:9).
SCREEN=${SCREEN:-1280x960}
SW=${SCREEN%x*}; SH=${SCREEN#*x}
CFG=$HOME/.android/avd/ashen_test.avd/config.ini
sed -i "/^hw.lcd.width=/d; /^hw.lcd.height=/d; /^hw.lcd.density=/d" "$CFG"
printf 'hw.lcd.width=%s
hw.lcd.height=%s
hw.lcd.density=240
' "$SW" "$SH" >> "$CFG"
nohup emulator -avd ashen_test -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -no-snapshot > /tmp/emu.log 2>&1 &
adb wait-for-device
until [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; do sleep 3; done
[ -n "$NOBUILD" ] || { bash scripts/build-android.sh > /tmp/abuild.log 2>&1 || { tail -30 /tmp/abuild.log; exit 1; }; }
adb install -r dist/AshenSanctum.apk
adb logcat -c
# Skip Android's one-time "Viewing full screen" tip (a real device shows it once).
adb shell settings put secure immersive_mode_confirmations confirmed
adb shell am start -n org.grayhatlabs.ashensanctum/.AshenActivity
sleep 25
adb exec-out screencap -p > dist/android_test.png
# Enter (a pad's A) into the menus, and a second screenshot.
adb shell input keyevent KEYCODE_ENTER
sleep 3
adb exec-out screencap -p > dist/android_test2.png
adb logcat -d -s SDL SDL/APP AndroidRuntime DEBUG | tail -60 > dist/android_log.txt || true
adb emu kill > /dev/null 2>&1 || true
echo "screenshot dist/android_test.png"

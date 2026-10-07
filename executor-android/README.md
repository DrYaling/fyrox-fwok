# FWOK Android package

This crate is the native-activity launcher used by `cargo-apk`. It is not a
desktop executable. Build and run it from the repository root with:

```powershell
rtk powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-android-package.ps1 -Run
```

The script copies `data/` into the Android asset staging directory, builds a
release APK, and installs/starts it when `adb` reports a connected device.

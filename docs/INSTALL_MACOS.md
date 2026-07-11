# Installing Zaroxi Studio on macOS

The macOS `.dmg` and `.pkg` builds are **not yet signed or notarized** with an
Apple Developer certificate. Gatekeeper will therefore refuse to open the app on
first launch with a message like *"Zaroxi Studio can't be opened because Apple
cannot check it for malicious software."* This is expected for an unsigned build
and does not indicate a problem with the download.

## Opening the app the first time

### From the `.dmg`

1. Open the downloaded `ZaroxiStudio-<version>-<arch>.dmg`.
2. Drag **Zaroxi Studio** into your **Applications** folder.
3. In `Applications`, **right-click (or Control-click)** the app and choose
   **Open**.
4. In the dialog, click **Open** again. macOS remembers this choice, so future
   launches work by double-clicking normally.

If the right-click → Open option is still blocked, allow it explicitly:

- Open **System Settings → Privacy & Security**.
- Scroll to the **Security** section — you should see a note that
  *"Zaroxi Studio was blocked"*. Click **Open Anyway**.

### From the `.pkg`

The installer places the app in `/Applications`. If the installer itself is
blocked, right-click the `.pkg` → **Open**, then follow the same
**Privacy & Security → Open Anyway** flow if prompted.

## Choosing the right architecture

- **Apple Silicon** (M1/M2/M3/M4 and later): use the `arm64` build.
- **Intel Macs**: use the `x86_64` build.

## For maintainers: enabling signing + notarization

Once a paid Apple Developer account is available, sign and notarize the app so
users no longer see Gatekeeper warnings:

1. Import the *Developer ID Application* certificate into the CI keychain.
2. `codesign --deep --force --options runtime --sign "Developer ID Application: …" "Zaroxi Studio.app"`
3. Submit for notarization with `xcrun notarytool submit … --wait`.
4. `xcrun stapler staple "Zaroxi Studio.app"` (and staple the `.dmg`).

These steps belong in the macOS `package-release` job once the signing secrets
(certificate, App Store Connect API key) are configured as repository secrets.

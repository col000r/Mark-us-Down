# Code Signing and Notarization Setup

To properly distribute your macOS app without the "damaged" warning, you need to set up code signing and notarization.

## Prerequisites

1. **Apple Developer Account** ($99/year)
2. **Developer Certificate** - Download from Apple Developer portal

## Required GitHub Secrets

Add these secrets to your GitHub repository (Settings → Secrets and variables → Actions):

### `APPLE_CERTIFICATE`
- Export your Developer ID Application certificate from Keychain as a .p12 file
- Convert to base64: `base64 -i certificate.p12 | pbcopy`
- Paste the base64 string as the secret value

### `APPLE_CERTIFICATE_PASSWORD`
- The password you used when exporting the .p12 certificate

### `APPLE_SIGNING_IDENTITY`
- Your certificate's Common Name, usually: `Developer ID Application: Your Name (TEAM_ID)`
- Find it by running: `security find-identity -v -p codesigning`

### `APPLE_ID`
- Your Apple ID email address

### `APPLE_PASSWORD`
- An App-Specific Password (not your regular Apple ID password)
- Generate at: https://appleid.apple.com/account/manage → Sign-In and Security → App-Specific Passwords

### `APPLE_TEAM_ID`
- Your 10-character Team ID from Apple Developer portal
- Found in your Apple Developer account under Membership

## Steps to Set Up

1. **Get certificates from Apple Developer portal**
2. **Export certificate as .p12 from Keychain Access**
3. **Generate App-Specific Password**
4. **Add all secrets to GitHub repository**
5. **Push changes and create a new release**

## Verification

After setup, your built DMG will be:
- Code signed with your Developer ID
- Notarized by Apple
- Installable without security warnings

The build process will automatically handle signing and notarization when these secrets are configured.
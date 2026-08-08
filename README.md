# Warp

Warp is an open-source, local-first file transfer tool for moving files between phones, tablets, and computers without uploading them to the cloud.

## Current release

Warp v0.1 Alpha supports a desktop running Warp as the local transfer host.

From a browser, nearby devices can both upload files to the computer and download files shared by the computer.

### Supported today

- iPhone / iPad ↔ Windows
- Android ↔ Windows
- Mac / Linux / other computers ↔ Windows
- macOS host builds are provided for Intel and Apple Silicon

Because the sender interface runs in a browser, no mobile app is required.

## How it works

Warp creates two folders:

```text
Downloads/Warp/
├── Received/
└── Share/
Files uploaded from another device are stored in Received.

Files placed in Share appear in the Warp web interface and can be downloaded by another device.

Warp streams files directly over the local network instead of uploading them to an external server.

Usage
Start Warp on your computer.
Warp displays a local address and a six-digit pairing code.
Connect the other device to the same local network.
Open the displayed address in Safari, Chrome, or another modern browser.
Enter the pairing code.
Upload or download files.

Windows Mobile Hotspot can also be used to create a direct local network between the computer and another device.

Security

Warp v0.1 is an alpha release.

Transfers currently use plain HTTP over the local network. The pairing code prevents casual unauthorized transfers but does not provide encrypted transport.

Use Warp v0.1 only on trusted local networks.

Encrypted authenticated transfers are planned.
# ⚡ Warp

**Fast, private, local-first file sharing across devices.**

Warp lets you transfer files between Windows, macOS, iPad, iPhone, Android, and other devices using your local Wi-Fi network — without uploading your files to the cloud.

No account. No subscription. No external server.

> **Current release: v0.1 Alpha** — A working early release. Some setup is still manual, and features are under active development.

## 📥 Download Warp

1. Visit the [Warp GitHub Releases](https://github.com/ankitsinghsftc-droid/warp/releases) page.
2. Download the ZIP file for your computer:

   | Platform | Download |
   |---|---|
   | Windows 64-bit | `Warp-Windows-x64.zip` |
   | macOS Apple Silicon (M1/M2/M3/M4) | `Warp-macOS-Apple-Silicon.zip` |
   | macOS Intel | `Warp-macOS-Intel.zip` |

3. Extract the ZIP into a folder of your choice.
4. Open a terminal in the extracted folder containing the Warp executable.

**You do not need Rust, Node.js, or the source code to use a prebuilt release.**

## 🚀 Getting Started

### Step 1 — Connect your devices

Connect both devices to the same local Wi-Fi network.

An internet connection is **not required** for file transfers.

Alternatively, on Windows, you can use Mobile Hotspot:

1. Open **Settings → Network & internet → Mobile hotspot**.
2. Turn on Mobile hotspot.
3. Connect your iPad, iPhone, or Android device to the laptop's hotspot.

This creates a local connection directly between your devices without requiring cloud services.

### Step 2 — Start Warp

**Windows**

Open PowerShell in the extracted folder and run:

```powershell
.\Warp.exe
```

**macOS**

Open Terminal in the extracted folder and run:

```bash
./Warp
```

On macOS, if the extracted executable lacks permission to run, use:

```bash
chmod +x ./Warp
./Warp
```

Warp will display the local connection addresses and a pairing code. In supported builds, a QR code or pairing link may also be shown.

Keep this terminal window open while transferring files.

### Step 3 — Connect your phone or iPad

On your second device:

1. Open Safari, Chrome, or another modern browser.
2. Enter the local address displayed by Warp in the terminal.

For example, when using Windows Mobile Hotspot, the address may be:

```text
http://192.168.137.1:8080
```

3. Enter the six-digit pairing code if requested.

If your Warp build displays a QR code, you can scan it to connect.

**No app installation is required on the receiving mobile device.**

## 📤 Transfer Files

### Phone / iPad → Computer

1. Open the Warp page in your mobile browser.
2. Choose the file you want to send.
3. Start the upload.
4. Wait for the transfer to complete.

Received files are stored in:

**Windows**
```text
Downloads\Warp\Received
```

**macOS**
```text
~/Downloads/Warp/Received
```

### Computer → Phone / iPad

1. Open the Warp `Share` folder on your computer.
2. Copy the files you want to share into that folder.
3. Open or refresh Warp in your mobile browser.
4. Select the file and download it.

**Windows**
```text
Downloads\Warp\Share
```

**macOS**
```text
~/Downloads/Warp/Share
```

Files are transferred through the local connection, rather than through cloud storage.

## ✨ Current Features

- Local-first file transfers
- Bidirectional computer ↔ mobile sharing
- Browser-based mobile access
- Six-digit pairing code
- Streaming file transfers for large files
- Automatic handling of duplicate filenames
- Temporary partial files during transfers
- Windows and macOS builds
- No account or cloud upload required

## 🛠 Troubleshooting

**Warp page isn't opening?**

Make sure both devices are connected to the same local network or that your phone/iPad is connected to the laptop hotspot. Verify that Warp is still running and use the IP address displayed in the terminal.

**Windows Firewall warning?**

Allow Warp on trusted private networks only.

**Transfer completed, but can't find the file?**

Check your computer's `Downloads/Warp/Received` folder, not the project source-code directory.

**Mobile device shows “No Internet Connection”?**

This can be normal when using an offline local hotspot. Warp does not need internet access to transfer files.

**Port 8080 is already in use?**

Close any previous Warp instance and restart the application.

## 🔐 Security Notice

Warp v0.1 Alpha uses local HTTP connections and pairing-code authentication.

**Transfers are not yet end-to-end encrypted.** Use Warp only on networks and with devices you trust. Avoid untrusted public Wi-Fi networks and do not expose Warp's port to the internet.

Encrypted communication and improved device authentication are planned.

## 🗺️ What's Next?

Warp is evolving beyond basic file transfers.

Planned improvements include:

- A native graphical desktop application
- QR-based instant pairing
- Automatic nearby-device discovery
- Direct offline device connections without manually configuring a hotspot
- Faster transfers and performance optimization
- Resume support for interrupted transfers
- Encrypted transfer protocols
- Warp Space — a private local sharing experience across devices

These features are under development and are not all available in the current public release.

## 👨‍💻 For Developers

If you want to build Warp from source:

```bash
git clone https://github.com/ankitsinghsftc-droid/warp.git
cd warp
cargo build --release --bin Warp
```

On Windows, run:

```powershell
.\target\release\Warp.exe
```

The `src/` directory contains Rust source code. End users downloading a release ZIP do not need to open or modify it.

## 🤝 Contributing

Warp is an open-source project built to make local file sharing simpler, faster, and more accessible.

Bug reports, feature suggestions, testing, and contributions are welcome.

Use [GitHub Issues](https://github.com/ankitsinghsftc-droid/warp/issues) to share feedback.

## 📄 License

Licensed under the **Apache License 2.0**.

---

**Warp — Your files. Your devices. Your network.**

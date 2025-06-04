# MajUSB Bootable Creator

A modern, cross-platform USB bootable drive creator written in Rust with a GTK4 GUI. Easily write Linux or Windows ISO images to USB drives with real-time progress, robust error handling, and a polished, user-friendly interface.

---

## Features
- **Modern GTK4 GUI**: Clean, responsive, and user-friendly interface.
- **Cross-platform**: Works on Linux (tested on major distros).
- **Write Linux & Windows ISOs**: Handles both Linux and Windows bootable USB creation.
- **Privilege Escalation**: Uses a secure helper binary with `pkexec` only when needed.
- **Real-time Log & Progress**: See detailed output and progress as the ISO is written.
- **Cluster Size Selection**: Choose NTFS cluster size for Windows ISOs.
- **Dependency Check**: Detects missing system packages and provides install instructions.
- **System Notifications**: Notifies you when the operation completes.
- **Auto-refresh Device List**: Detects USB device changes automatically.
- **Robust Error Handling**: Clear guidance and error messages.

---

## Repository
- GitHub: [https://github.com/vicrodh/usb-bootable-creator](https://github.com/vicrodh/usb-bootable-creator)
- SSH: `git@github.com:vicrodh/usb-bootable-creator.git`

---

## Prerequisites

### 1. Install Rust (if not already installed)

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Follow the prompts, then restart your terminal or run:
source $HOME/.cargo/env
```

### 2. Install GTK4 and Required System Packages

<details>
<summary><strong>Arch Linux / Manjaro / EndeavourOS / Artix / Garuda / SteamOS</strong></summary>

```sh
sudo pacman -S --needed base-devel rustup gtk4 glib2 gio util-linux coreutils dosfstools ntfs-3g parted rsync polkit
```
</details>

<details>
<summary><strong>Debian / MX Linux / antiX</strong></summary>

```sh
sudo apt update
sudo apt install -y build-essential rustup libgtk-4-dev libglib2.0-dev libgio2.0-dev util-linux coreutils dosfstools ntfs-3g parted rsync policykit-1
```
</details>

<details>
<summary><strong>Ubuntu / Linux Mint / Pop!_OS / Zorin / Kubuntu / Lubuntu / Xubuntu / Elementary</strong></summary>

```sh
sudo apt update
sudo apt install -y build-essential rustup libgtk-4-dev libglib2.0-dev libgio2.0-dev util-linux coreutils dosfstools ntfs-3g parted rsync policykit-1
```
</details>

<details>
<summary><strong>Fedora / Nobara / Bazzite</strong></summary>

```sh
sudo dnf install -y @development-tools rust gtk4-devel glib2-devel gio-devel util-linux coreutils dosfstools ntfs-3g parted rsync polkit
```
</details>

<details>
<summary><strong>openSUSE (Leap, Tumbleweed, GeckoLinux)</strong></summary>

```sh
sudo zypper install -y rust gtk4-devel glib2-devel gio-devel util-linux coreutils dosfstools ntfs-3g parted rsync polkit
```
</details>

<details>
<summary><strong>Alpine Linux</strong></summary>

```sh
sudo apk add build-base rustup gtk4-dev glib-dev gio-dev lsblk coreutils dosfstools ntfs-3g-progs parted rsync polkit
```
</details>

<details>
<summary><strong>Void Linux</strong></summary>

```sh
sudo xbps-install -S base-devel rustup gtk4-devel glib-devel gio-devel util-linux coreutils dosfstools ntfs-3g parted rsync polkit
```
</details>

<details>
<summary><strong>Gentoo</strong></summary>

```sh
sudo emerge --ask sys-devel/gcc sys-devel/make sys-apps/util-linux sys-apps/coreutils sys-fs/dosfstools sys-fs/ntfs3g sys-block/parted net-misc/rsync sys-auth/polkit x11-libs/gtk+:4 dev-libs/glib dev-libs/gio dev-lang/rust
```
</details>

<details>
<summary><strong>NixOS</strong></summary>

```sh
nix-env -iA nixos.gcc nixos.make nixos.util-linux nixos.coreutils nixos.dosfstools nixos.ntfs3g nixos.parted nixos.rsync nixos.polkit nixos.gtk4 nixos.glib nixos.gio nixos.rustc
```
</details>

<details>
<summary><strong>Other Distributions</strong></summary>

Install the following packages (names may vary):
- build tools (gcc, make, etc.)
- rustup
- gtk4, glib, gio development libraries
- util-linux, coreutils, dosfstools, ntfs-3g, parted, rsync, polkit

</details>

---

## Building (Arch Linux Example)

```sh
git clone https://github.com/vicrodh/usb-bootable-creator.git
cd usb-bootable-creator
cargo build --release
```

- The main GUI binary will be in `target/release/rust-usb-bootable-creator`.
- The privilege helper will be in `target/release/cli_helper`.

---

## Running

```sh
cargo run --release
# Or run the built binary directly:
./target/release/rust-usb-bootable-creator
```

### Using the App
- Select an ISO file.
- Select a USB device from the list.
- (Optional) Select cluster size for Windows ISOs.
- Click "Write" and confirm the operation.
- Watch the real-time log and progress bar.
- Wait for the system notification on completion.

---

## Notes
- **Privilege escalation**: The app uses `pkexec` to run a helper binary (`cli_helper`) for writing to USB devices. You may be prompted for your password.
- **Dependency check**: On startup, the app checks for required system packages and will show a dialog with install instructions if anything is missing.
- **Windows support**: Native Windows support is planned but not yet implemented. For now, use on Linux.

---

## Troubleshooting
- If the app fails to start, ensure all dependencies are installed (see above).
- If USB devices do not appear, try re-plugging the device or running the app with appropriate permissions.
- For issues with writing Windows ISOs, ensure `wimlib-imagex` is installed.
- For any other issues, check the real-time log output for details.

---

## Project Structure
- `src/gui.rs` — Main GTK4 GUI logic
- `src/utils.rs` — Device listing, privilege escalation, OS detection, dependency check
- `src/flows/` — ISO writing logic for Linux and Windows
- `src/bin/cli_helper.rs` — Helper binary for privileged operations
- `Cargo.toml` — Project manifest and dependencies

---

## License
MIT License. See [LICENSE](LICENSE) for details.

---

## Credits
- Developed by Vic RH
- Inspired by open-source USB creation tools like Rufus. :) 

---

## Contributing
Pull requests and issues are welcome! Please open an issue for bugs or feature requests.

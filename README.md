# 💻 dotfiles

<p align="center">
  <img src="https://img.shields.io/badge/Language-Rust-orange?style=for-the-badge&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/OS-Windows-0078D4?style=for-the-badge&logo=windows&logoColor=white" alt="Windows" />
  <img src="https://img.shields.io/badge/License-MIT-415A77?style=for-the-badge" alt="MIT License" />
  <img src="https://img.shields.io/badge/Version-1.0.0-8338EC?style=for-the-badge" alt="Version 1.0.0" />
</p>

```text
     _       _   __ _ _
  __| | ___ | |_/ _(_) | ___  ___
 / _` |/ _ \| __| |_| | |/ _ \/ __|
| (_| | (_) | |_|  _| | |  __/\__ \
 \__,_|\___/ \__|_| |_|_|\___||___/
```

### 📋 About
**dotfiles** is a lightweight, high-performance command-line utility built in Rust to manage your dotfiles on Windows. It centralization mirrors configuration files and folders from your `$USERPROFILE` directory into a secure `.dotfiles/` directory, using native symbolic links and directory junctions to keep configuration paths active.

---

### ⚙️ Windows Symlink Requirement
By default, creating symbolic links in Windows requires elevated administrator permissions. To run `dotfiles` without Administrator rights:
1. Open **Settings** (Win + I).
2. Go to **Update & Security** > **For developers** (or search "Developer settings").
3. Toggle **Developer Mode** to **On**.

*Otherwise, please run your terminal/shell as **Administrator**.*

---

### 🛠 Visual Workflow

```text
 ┌───────────────── USERPROFILE ─────────────────┐
 │                                               │
 │    .gitconfig              .config/           │
 │  (Symlink File)      (Directory Junction)     │
 └───────┬───────────────────────┬───────────────┘
         │                       │
         └───────────┬───────────┘
                     ▼
       ┌───────── .dotfiles/ ──────────┐
       │                               │
       │    .gitconfig   .config/      │
       │  (True Backup) (True Backup)  │
       │                               │
       │       .dot (Metadata JSON)    │
       └───────────────────────────────┘
```

---

### 📦 Installation
Ensure you have Rust/Cargo installed, then run:
```bash
# Install directly from repository path
cargo install --path .
```

---

### 🕹 Usage Guide
All commands are simple and clean:
```text
Usage:
  dotfiles link <paths...>      Backup dotfiles and replace with symlink/junction
  dotfiles unlink <paths...>    Restore original files from backup
  dotfiles check                 Verify health and self-heal missing links
  dotfiles list                  List active dotfiles and targets
  dotfiles list --backup         List all backed up files in metadata database
  dotfiles help                  Display help guide
```

#### Examples
- **Link configurations**:
  `dotfiles link .gitconfig .config`
- **Verify health & self-heal**:
  `dotfiles check`
- **Check status list**:
  `dotfiles list`

---

### 📜 License
Licensed under the **MIT** License.

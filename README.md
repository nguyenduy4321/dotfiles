# 💻 dotfiles

```text
     _       _   __ _ _
  __| | ___ | |_/ _(_) | ___  ___
 / _` |/ _ \| __| |_| | |/ _ \/ __|
| (_| | (_) | |_|  _| | |  __/\__ \
 \__,_|\___/ \__|_| |_|_|\___||___/
```

> **A lightweight, fast, Windows-first command-line utility written in Rust to seamlessly manage, backup, and restore your dotfiles.**

---

## ⚡ Quick Start

```bash
# 1. Install dotfiles
cargo install --path .

# 2. Link your configuration files
dotfiles link .gitconfig .config

# 3. Check health status
dotfiles check
```

---

## 🛠 System Overview & Architecture

```text
                 [ USERPROFILE ]
            (e.g., C:\Users\Username)
                       │
         ┌─────────────┴─────────────┐
         ▼                           ▼
    .gitconfig                    .config
   (Symlink File)           (Directory Junction)
         │                           │
         └─────────────┬─────────────┘
                       ▼
                 [ .dotfiles/ ]
            (Backup & Storage Depot)
                       │
                       ▼
                 [ .dot metadata ]
```

---

## 🌟 Key Features

*   **⚡ Zero Config**
    Scans files/directories automatically under `$USERPROFILE` (must start with `.`).
*   **🪟 Windows-First Design**
    Uses directory junctions for directories and symbolic links for files. Works without needing elevated admin shell permissions.
*   **🛡️ Self-Healing (`check`)**
    Automatic sync & check. Recreates missing links if the backup is healthy, or purges metadata if the backup is missing.
*   **💾 Cross-Device Safety**
    Includes a robust fallback copying mechanism when migrating configs across different disk volumes.

---

## 🕹 Usage & CLI Commands

```text
Usage:
  dotfiles link <paths...>      Backup dotfile and replace with symlink/junction
  dotfiles unlink <paths...>    Restore original files from backup
  dotfiles check                 Verify integrity and self-heal
  dotfiles list                  List dotfiles under USERPROFILE and link status
  dotfiles list --backup         List dotfiles stored in metadata backup
  dotfiles help                  Show help guidelines
```

### Examples

#### 🔗 Linking
```bash
$ dotfiles link .gitconfig .vimrc
Moving .gitconfig -> .dotfiles\.gitconfig... Done!
Creating symlink for .gitconfig... Done!
```

#### 🔍 Checking Status
```bash
$ dotfiles list
NAME         TYPE       LINK       TARGET
----------------------------------------------------
.gitconfig   file       symlink    C:\path\to\.dotfiles\.gitconfig
.config      directory  junction   C:\path\to\.dotfiles\.config
```

---

## 📦 Build & Installation

```bash
# Clone the repository
git clone https://github.com/nguyenduy4321/dotfiles.git
cd dotfiles

# Build release profile
cargo build --release
```

The optimized binary will be available at `./target/release/dotfiles.exe`.

---

## 📜 License

Distributed under the **MIT** License. See `LICENSE` for more information.

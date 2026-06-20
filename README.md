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

**dotfiles** is an ultra-fast, Windows-first command-line utility built in Rust to manage your dotfiles without hassle. It uses Windows **directory junctions** and **file symbolic links** to safely mirror your configuration files from `$USERPROFILE` directly into a centralized backup folder (`.dotfiles/`), protecting your data while maintaining compatibility.

---

### ⚡ Quick Command Cheat Sheet

```bash
# 📦 Add & backup configuration files
dotfiles link .gitconfig .config

# 🔄 Restore configurations to their original state
dotfiles unlink .gitconfig

# 🩺 Run integrity diagnostics (Self-Healing)
dotfiles check

# 🔍 View current managed files & backup statuses
dotfiles list
```

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

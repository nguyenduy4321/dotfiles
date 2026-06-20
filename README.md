# dotfiles

A lightweight, fast, Windows-first command-line tool written in Rust to manage, backup, and sync your dotfiles.

## Features
- **Zero Configuration**: Detects your dotfiles automatically under `$USERPROFILE` (must start with a `.`).
- **Windows-First Design**: Uses Windows **directory junctions** for directories and **symbolic links** for files, ensuring compatibility without requiring elevated administrator shell access in many cases.
- **Cross-Device Recovery**: Robust fallback copying mechanism when moving items between different disk drives.
- **Self-Healing Integrity Check (`check`)**:
  - Automatically restores missing symlinks/junctions if the backup exists.
  - Automatically cleans up the link and removes the entry from metadata if the backup is missing.
- **Robust List command**: Display details of active and backed up dotfiles, with clear metadata differentiating files/directories and symlinks/junctions.

## Installation
Ensure you have Rust and Cargo installed, then build:
```bash
cargo build --release
```

## Usage
All operations are relative to your `$USERPROFILE` directory.

### 1. Link a dotfile / directory
Backup your original dotfile and replace it with a symlink (or junction for directories).
```bash
dotfiles link .gitconfig .config
```

### 2. Unlink / Restore
Restore original files from the backup directory and remove the link.
```bash
dotfiles unlink .gitconfig
```

### 3. Check Integrity (Self-Healing)
Verify the state of all managed dotfiles. Recreates missing links or removes orphan metadata.
```bash
dotfiles check
```

### 4. List Dotfiles
Show all dotfiles in your profile and their status:
```bash
dotfiles list
```
Or show all currently backed-up dotfiles:
```bash
dotfiles list --backup
```

## License
MIT

================================================================================
     _       _   __ _ _
  __| | ___ | |_/ _(_) | ___  ___
 / _` |/ _ \| __| |_| | |/ _ \/ __|
| (_| | (_) | |_|  _| | |  __/\__ \
 \__,_|\___/ \__|_| |_|_|\___||___/
================================================================================
dotfiles - A Windows-First command-line utility written in Rust to manage, 
backup, and restore your configuration files.

[ ABOUT ]
dotfiles is a high-performance utility that automatically links files and
folders in your Windows USERPROFILE directory to a centralized backup folder
named `.dotfiles/`. It uses native symlinks for files and junctions for
directories, keeping your workspace clean and safe.

[ QUICK START ]
1. Run PowerShell as Administrator (only needed for file symlinks if Developer
   Mode is not enabled on your system).
2. To link/backup a configuration file or folder:
   dotfiles link .gitconfig .config
3. To unlink/restore the files to their original locations:
   dotfiles unlink .gitconfig
4. To check/verify integrity and self-heal broken links:
   dotfiles check
5. To list all active or backed up dotfiles:
   dotfiles list
   dotfiles list --backup

[ COMMAND SUMMARY ]
  link <paths...>     Backup target dotfiles and replace with symlink/junction.
  unlink <paths...>   Restore original configurations and delete links.
  check               Run self-healing on broken links/metadata.
  list                Show active dotfile status in your USERPROFILE.
  list --backup       Show backed up files in metadata backup.
  help                Display the CLI help guide.

[ LICENSE ]
MIT License.
================================================================================

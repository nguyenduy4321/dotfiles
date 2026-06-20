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
dotfiles is a high-performance utility that centralization mirrors configuration
files and folders from your Windows USERPROFILE directory into a secure
`.dotfiles/` directory, using native symlinks for files and junctions for
directories.

[ WINDOWS SYMLINK REQUIREMENT ]
By default, creating symbolic links in Windows requires elevated administrator
permissions. To run `dotfiles` without Administrator rights:
1. Open Settings (Win + I).
2. Go to Update & Security > For developers.
3. Toggle "Developer Mode" to On.

Otherwise, please run your terminal/shell as Administrator.

[ QUICK START ]
1. To link/backup a configuration file or folder:
   dotfiles link .gitconfig .config
2. To unlink/restore the files to their original locations:
   dotfiles unlink .gitconfig
3. To check/verify integrity and self-heal broken links:
   dotfiles check
4. To list all active or backed up dotfiles:
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

use dotfiles::cmd::{run_check, run_link, run_list, run_unlink};
use dotfiles::core::EnvCtx;
use std::env;
use std::path::PathBuf;

fn print_help() {
    println!(
        "dotfiles - A Windows CLI to manage and backup dotfiles\n\nUsage:\n  dotfiles link <paths...>          # Backup each dotfile and replace with a symlink (file) or junction (dir)\n  dotfiles unlink <paths...>        # Restore original files from backup and remove links\n  dotfiles check                     # Verify integrity; recreates missing links if backup exists, removes entries if backup missing\n  dotfiles list                      # List dotfiles present in your USERPROFILE with link status\n  dotfiles list --backup             # List dotfiles that are currently backed up (metadata)\n  dotfiles help                      # Show this help message\n\nOptions:\n  <paths...>   One or more dotfiles (must start with a '.') relative to $USERPROFILE or absolute paths inside $USERPROFILE.\n\nExamples:\n  dotfiles link .gitconfig .vimrc    # Manage two config files\n  dotfiles unlink .gitconfig         # Restore original config\n  dotfiles check                     # Run verification and clean up broken entries\n  dotfiles list --backup             # See which dotfiles are currently stored in .dotfiles\n\nNotes:\n  • A dotfile is any file or directory whose name starts with a '.' (dot).\n  • Backups are stored in the executable's directory under .dotfiles/.\n  • Links are created as symlinks on files and directory junctions on Windows for directories.\n  • If a dotfile is already managed, linking it again prints a notice and does nothing.\n  • Unlinking an un-managed path prints a notice and does nothing.\n  • The check command recreates missing/broken links if the backup exists, and deletes links/metadata if the backup is missing, reporting each action."
    );
}

fn main() {
    let user_profile = env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default"));
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let ctx = EnvCtx {
        user_profile,
        exe_dir,
    };

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    let cmd = args[1].as_str();
    let res = match cmd {
        "help" => {
            print_help();
            Ok(())
        }
        "link" => {
            if args.len() < 3 {
                Err("No paths provided for linking".to_string())
            } else {
                run_link(&ctx, &args[2..])
            }
        }
        "unlink" => {
            if args.len() < 3 {
                Err("No paths provided for unlinking".to_string())
            } else {
                run_unlink(&ctx, &args[2..])
            }
        }
        "check" => run_check(&ctx),
        "list" => {
            let show_backup = args.len() >= 3 && args[2] == "--backup";
            run_list(&ctx, show_backup)
        }
        _ => {
            print_help();
            return;
        }
    };

    if let Err(e) = res {
        eprintln!("ERROR:\n{}", e);
        std::process::exit(1);
    }
}

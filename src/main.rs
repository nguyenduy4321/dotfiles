use dotfiles::cmd::{run_check, run_link, run_list, run_unlink};
use dotfiles::core::EnvCtx;
use std::env;
use std::path::PathBuf;

fn print_help() {
    println!(
        "\x1b[1;36m     _       _   __ _ _             \x1b[0m\n\
         \x1b[1;36m  __| | ___ | |_/ _(_) | ___  ___   \x1b[0m\n\
         \x1b[1;36m / _` |/ _ \\| __| |_| | |/ _ \\/ __|  \x1b[0m\n\
         \x1b[1;36m| (_| | (_) | |_|  _| | |  __/\\__ \\  \x1b[0m\n\
         \x1b[1;36m \\__,_|\\___/ \\__|_| |_|_|\\___||___/  \x1b[0m\n\n\
         \x1b[1;32mdotfiles\x1b[0m - A Windows CLI to manage and backup dotfiles\n\n\
         \x1b[1;33mUSAGE:\x1b[0m\n  \
           dotfiles \x1b[1;36mlink\x1b[0m <paths...>          # Backup dotfiles and replace with symlink/junction\n  \
           dotfiles \x1b[1;36munlink\x1b[0m <paths...>        # Restore files from backup and delete links\n  \
           dotfiles \x1b[1;36mcheck\x1b[0m                     # Self-heal links and clean orphan metadata\n  \
           dotfiles \x1b[1;36mlist\x1b[0m                      # List dotfiles in profile with active link status\n  \
           dotfiles \x1b[1;36mlist --backup\x1b[0m             # List backed up dotfiles inside metadata\n  \
           dotfiles \x1b[1;36mhelp\x1b[0m                      # Show this help guide\n\n\
         \x1b[1;33mOPTIONS:\x1b[0m\n  \
           \x1b[35m<paths...>\x1b[0m  One or more dotfiles starting with '.' in $USERPROFILE\n\n\
         \x1b[1;33mEXAMPLES:\x1b[0m\n  \
           dotfiles link .gitconfig .vimrc\n  \
           dotfiles unlink .gitconfig\n  \
           dotfiles check\n  \
           dotfiles list --backup\n\n\
         \x1b[1;33mNOTES:\x1b[0m\n  \
           • Backups are stored in the executable's directory under \x1b[32m.dotfiles/\x1b[0m\n  \
           • Links are symlinks on files and directory junctions on Windows\n  \
           • Re-linking already managed files is safely ignored"
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

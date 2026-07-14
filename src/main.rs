use clap::Parser;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::env::consts::OS;
use walkdir::WalkDir;
use zip::read::ZipArchive;

#[derive(Parser)]
#[command(name = "RMCPT")]
struct Cli {
    #[arg(short, long)] platform: String,
    #[arg(short, long)] base: String,
    #[arg(short, long)] address: String,
}

const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

fn print_header() {
    println!("{RED}{}",
        r#" ________  ________  _____ ______   ________  _________
|\   __  \|\   ____\|\   _ \  _   \|\   __  \|\___   ___\
\ \  \|\  \ \  \___|\ \  \\\__\ \  \ \  \|\  \|___ \  \_|
 \ \   _  _\ \  \    \ \  \\|__| \  \ \   ____\   \ \  \
  \ \  \\  \\ \  \____\ \  \    \ \  \ \  \___|    \ \  \
   \ \__\\ _\\ \_______\ \__\    \ \__\ \__\        \ \__\
    \|__|\|__|\|_______\|\__|     \|__|\|__|         \|__|

Revival Mobile Client Patching Tool
made w <3 by vancy
"#);
}

fn main() {
    let args = Cli::parse();

    if args.address.len() != 10 {
        std::process::exit(1);
    }

    print_header();

    match args.platform.to_lowercase().as_str() {
        "android" => handle_android(&args.base, &args.address),
        "ios" => handle_ios(&args.base, &args.address),
        _ => eprintln!("Unsupported platform"),
    }
}

fn unzip_internal(file: &str, dest: &str) {
    let file = fs::File::open(file).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    archive.extract(dest).unwrap();
}

fn zip_directory(src: &str, output: &str) {
    match OS {
        "windows" => {
            let zip_temp = output.replace(".xapk", ".zip").replace(".ipa", ".zip");

            Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "Compress-Archive -Path '{}/*' -DestinationPath '{}' -Force",
                        src, zip_temp
                    ),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .ok();

            fs::rename(zip_temp, output).ok();
        }

        _ => {
            Command::new("zip")
                .args(["-r", output, "."])
                .current_dir(src)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .ok();
        }
    }
}

fn patch_directory(dir: &str, old: &str, new: &str) {
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                if content.contains(old) {
                    let new_content = content.replace(old, new);
                    let _ = fs::write(path, new_content);
                }
            }
        }
    }
}

fn handle_android(base_path: &str, addr: &str) {
    let is_xapk = Path::new(base_path)
        .extension()
        .and_then(|s| s.to_str())
        == Some("xapk");

    let work_dir = "temp_work";
    let decompile_dir = "temp_decompiled";

    let _ = fs::remove_dir_all(work_dir);
    let _ = fs::remove_dir_all(decompile_dir);
    let _ = fs::create_dir_all(work_dir);

    println!("{RED}[*]{RESET} Extracting...");
    unzip_internal(base_path, work_dir);

    let apk_to_patch = WalkDir::new(work_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("apk"))
        .expect("No APK found!")
        .path()
        .to_str()
        .unwrap()
        .to_string();

    println!("{RED}[*]{RESET} Decompiling app...");

    Command::new("java")
        .args([
            "-jar",
            "dependencies/apktool.jar",
            "d",
            &apk_to_patch,
            "-o",
            decompile_dir,
            "-f",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok();

    println!("{RED}[+]{RESET} Applying changes...");
    patch_directory(decompile_dir, "roblox.comin", addr);

    println!("{RED}[*]{RESET} Recompiling app...");

    Command::new("java")
        .args([
            "-jar",
            "dependencies/apktool.jar",
            "b",
            decompile_dir,
            "-o",
            "patched.apk",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok();

    println!("{RED}[*]{RESET} Signing app...");

    Command::new("java")
        .args([
            "-jar",
            "dependencies/uber-apk-signer.jar",
            "--apks",
            "patched.apk",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok();

    if is_xapk {
        println!("{RED}[*]{RESET} Rezipping...");

        fs::copy(
            "patched-aligned-debugSigned.apk",
            format!("{}/base.apk", work_dir),
        )
        .ok();

        zip_directory(work_dir, "patched.xapk");

        println!("\nSuccess: patched.xapk patched");
    } else {
        fs::rename("patched-aligned-debugSigned.apk", "patched.apk").ok();
        println!("\nSuccess: patched.apk patched");
    }
}

fn handle_ios(base: &str, addr: &str) {
    let output_dir = "ios_extracted";

    let _ = fs::remove_dir_all(output_dir);

    println!("{RED}[*]{RESET} Extracting...");
    unzip_internal(base, output_dir);

    println!("{RED}[+]{RESET} Applying changes...");
    patch_directory(output_dir, "roblox.comin", addr);

    println!("{RED}[*]{RESET} Rezipping...");
    zip_directory(output_dir, "patched.ipa");

    println!("\nSuccess: patched.ipa patched");
}

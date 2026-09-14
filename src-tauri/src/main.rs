use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let handled = match args[1].as_str() {
            "--toggle" | "-s" | "--search" | "--toggle-search" | "search" | "open-search"
            | "toggle" => searchis_lib::send_cli_command("toggle"),
            "--manager" | "manager" | "show" => searchis_lib::send_cli_command("show-manager"),
            "--settings" | "settings" => searchis_lib::send_cli_command("settings"),
            "--quit" | "quit" => searchis_lib::send_cli_command("quit"),
            _ => false,
        };
        if handled {
            return;
        }
    } else if env::var("SEARCHIS_E2E").is_err() && searchis_lib::send_cli_command("show-manager") {
        // 无参数启动时，如果已有实例在后台运行，唤出其管理窗口，避免重复启动
        return;
    }

    searchis_lib::run();
}

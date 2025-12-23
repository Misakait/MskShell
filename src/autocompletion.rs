use std::env;
use std::fs;
use std::fs::DirEntry;

use crate::trie::Trie;

pub fn collect_all_commands() -> Trie {
    let mut commands = Trie::new();

    // 1. 加入内置命令
    let builtins = vec!["echo", "exit", "type", "history", "pwd", "cd"];
    for b in builtins {
        commands.insert(b);
    }

    // 2. 扫描 PATH 环境变量
    if let Some(paths) = env::var_os("PATH") {
        env::split_paths(&paths)
            .flat_map(|dir| fs::read_dir(dir).into_iter().flatten())
            .flatten() // 解包 Result<DirEntry>
            .filter(|entry| is_entry_executable(entry))
            // 最后才转换文件名，这是无法避免的分配，但只有合法的可执行文件才会走到这一步
            .for_each(|entry| {
                let os_name = entry.file_name();
                if let Some(name_str) = os_name.to_str() {
                    commands.insert(name_str);
                }
            });
    }
    commands
}
pub fn is_entry_executable(entry: &DirEntry) -> bool {
    // 1. 第一道防线：文件类型检查 (极快)
    // entry.file_type() 在大多数现代 Unix (如 Linux) 上是不需要额外系统调用的
    // 因为 readdir() 已经顺便把文件类型带回来了 (d_type)。
    match entry.file_type() {
        Ok(ft) if ft.is_file() => {
            // 继续检查权限
        }
        // 如果是目录或者获取失败，直接返回 false，不仅省了 metadata 调用，还省了后续逻辑
        _ => return false,
    }

    // 2. 第二道防线：权限/扩展名检查
    #[cfg(unix)]
    {
        // entry.metadata() 比 fs::metadata(path) 稍微快一点，因为不需要解析路径
        if let Ok(metadata) = entry.metadata() {
            // 0o111 代表 owner, group, other 任意一个有 x 权限

            use std::os::unix::fs::PermissionsExt;
            return metadata.permissions().mode() & 0o111 != 0;
        }
    }

    #[cfg(windows)]
    {
        // Windows 下主要看扩展名。
        // entry.file_name() 返回 &OsStr，这里是零拷贝引用的
        let name = entry.file_name();
        // 转换损耗极小，只在非 ASCII 字符时有微小开销
        let name_str = name.to_string_lossy();

        // 检查后缀
        if let Some(ext_idx) = name_str.rfind('.') {
            let ext = &name_str[ext_idx + 1..];
            // 常见的可执行后缀，按命中概率排序
            const EXEC_EXTS: &[&str] = &["exe", "bat", "cmd", "ps1", "com"];
            // 忽略大小写比较
            return EXEC_EXTS.iter().any(|&e| e.eq_ignore_ascii_case(ext));
        }
    }

    false
}
pub fn longest_common_prefix(strings: &[String]) -> Option<String> {
    if strings.is_empty() {
        return None;
    }
    // 拿第一个字符串做基准
    let mut prefix = strings[0].clone();
    for s in &strings[1..] {
        while !s.starts_with(&prefix) {
            // 如果不匹配，就缩短前缀，直到匹配为止
            prefix.pop();
            if prefix.is_empty() {
                return None;
            }
        }
    }
    Some(prefix)
}

use std::mem;

#[derive(Debug, PartialEq)]
pub enum Token {
    Op(String),
    Word(Vec<Args>),
}

#[derive(Debug, PartialEq)]
pub enum Args {
    Raw(String),
    SingleQuotes(String),
    DoubleQuotes(String),
}

#[derive(PartialEq)]
enum ParseState {
    Normal,
    InSingleQuotes,
    InDoubleQuotes,
}
fn char_is_op(c: char) -> bool {
    matches!(c, '>' | '|')
}
pub fn tokens_generate(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut chars = input.chars().peekable();

    // 当前单词的组件列表
    let mut current_word_args: Vec<Args> = Vec::new();
    // 当前正在构建的字符串
    let mut current_string = String::new();

    let mut state = ParseState::Normal;

    let mut flush_string_to_args =
        |args_vec: &mut Vec<Args>, s: &mut String, state: &ParseState| {
            if !s.is_empty() {
                let value_to_push = mem::take(s);
                match state {
                    ParseState::Normal => args_vec.push(Args::Raw(value_to_push)),
                    ParseState::InSingleQuotes => args_vec.push(Args::SingleQuotes(value_to_push)),
                    ParseState::InDoubleQuotes => args_vec.push(Args::DoubleQuotes(value_to_push)),
                }
            }
        };

    // --- 辅助闭包：负责把 Level 2 冲刷到 Level 3 ---
    let mut flush_args_to_token = |tokens_vec: &mut Vec<Token>, args_vec: &mut Vec<Args>| {
        if !args_vec.is_empty() {
            tokens_vec.push(Token::Word(mem::take(args_vec)));
        }
    };

    while let Some(c) = chars.next() {
        match state {
            // === 状态 1: 普通模式 (Raw) ===
            ParseState::Normal => {
                match c {
                    // 转义
                    '\\' => {
                        if let Some(next_char) = chars.next() {
                            current_string.push(next_char);
                        }
                    }
                    // 单引号：先结算之前的 Raw，再切换状态
                    '\'' => {
                        flush_string_to_args(&mut current_word_args, &mut current_string, &state);
                        state = ParseState::InSingleQuotes;
                    }
                    // 双引号：同上
                    '\"' => {
                        flush_string_to_args(&mut current_word_args, &mut current_string, &state);
                        state = ParseState::InDoubleQuotes;
                    }
                    // 空格：这是单词的分界线
                    c if c.is_whitespace() => {
                        // 1. 先把手里剩下的字符存入 args
                        flush_string_to_args(&mut current_word_args, &mut current_string, &state);
                        // 2. 再把 args 打包成 Token::Word
                        flush_args_to_token(&mut tokens, &mut current_word_args);
                    }
                    // 操作符 (>, |)：这也是单词的分界线！
                    c if char_is_op(c) => {
                        // 1. 类似空格，先结算前面的单词 (比如 ls>file 中的 ls)
                        flush_string_to_args(&mut current_word_args, &mut current_string, &state);
                        flush_args_to_token(&mut tokens, &mut current_word_args);

                        // 2. 处理操作符本身
                        let mut op = c.to_string();
                        if c == '>' && chars.peek() == Some(&'>') {
                            chars.next(); // 吃掉第二个 >
                            op.push('>');
                        }

                        // 3. 生成 Op Token
                        tokens.push(Token::Op(op));
                    }
                    // 数字：可能是 1> 或 2>，也可能是普通字符 123
                    c if c.is_digit(10) => {
                        // 1. 创建一个克隆的迭代器用于“侦察”
                        // 这不会消耗原本的 chars
                        let mut lookahead = chars.clone();

                        // 2. 统计连续数字的个数，除了c
                        let mut digit_count = 0;
                        while let Some(&n) = lookahead.peek() {
                            if n.is_digit(10) {
                                digit_count += 1;
                                lookahead.next();
                            } else {
                                break;
                            }
                        }

                        // 3. 检查数字跑完之后，紧接着的是不是 >
                        let is_redirect = matches!(lookahead.peek(), Some(&'>'));

                        // 4. 【关键判断】只有当 current_string 为空时，这才是 FD 重定向！
                        // 比如：
                        // "echo 1>" -> current_string 空，1> 是重定向
                        // "echo file1>" -> current_string 是 "file"，1> 只是文件名的一部分
                        if is_redirect && current_string.is_empty() {
                            // === 是重定向符 (如 10>) ===

                            // A. 收集所有的数字
                            let mut fd_str = c.to_string();
                            for _ in 0..digit_count {
                                fd_str.push(chars.next().unwrap()); // 真的消耗掉
                            }

                            // B. 消耗掉那个 >
                            chars.next();

                            // C. 检查是不是 >> (追加)
                            let mut op = format!("{}>", fd_str);
                            if let Some(&'>') = chars.peek() {
                                chars.next();
                                op.push('>'); // 变成 10>>
                            }

                            // D. 结算之前的并在 tokens 里加入 Op
                            //  current_string 为空
                            // flush_string_to_args(
                            //     &mut current_word_args,
                            //     &mut current_string,
                            //     &state,
                            // );
                            flush_args_to_token(&mut tokens, &mut current_word_args);

                            tokens.push(Token::Op(op));
                        } else {
                            // === 不是重定向符 (只是普通数字 123) ===
                            // 或者是 file1> 这种情况
                            current_string.push(c);
                            // 注意：我们只消耗了 c，并没有消耗后面那些 lookahead 里的数字
                            // 它们会在下几次循环中被处理
                        }
                    }
                    // 普通字符
                    _ => {
                        current_string.push(c);
                    }
                }
            }

            // === 状态 2: 单引号模式 ===
            ParseState::InSingleQuotes => {
                match c {
                    '\'' => {
                        // 结算单引号内容
                        flush_string_to_args(&mut current_word_args, &mut current_string, &state);
                        state = ParseState::Normal; // 回到普通模式
                    }
                    _ => current_string.push(c),
                }
            }

            // === 状态 3: 双引号模式 ===
            ParseState::InDoubleQuotes => {
                match c {
                    '\"' => {
                        // 结算双引号内容
                        flush_string_to_args(&mut current_word_args, &mut current_string, &state);
                        state = ParseState::Normal;
                    }
                    '\\' => {
                        // 双引号内的转义逻辑 (同之前)
                        match chars.peek() {
                            Some(&'\\') | Some(&'\"') | Some(&'$') | Some(&'\n') | Some(&'`') => {
                                current_string.push(chars.next().unwrap());
                            }
                            _ => current_string.push('\\'),
                        }
                    }
                    _ => current_string.push(c),
                }
            }
        }
    }

    // 循环结束后的最后一次结算
    flush_string_to_args(&mut current_word_args, &mut current_string, &state);
    flush_args_to_token(&mut tokens, &mut current_word_args);

    tokens
}

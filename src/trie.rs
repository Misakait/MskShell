use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct Trie {
    children: HashMap<char, Trie>,
    is_leaf: bool,
}

impl Trie {
    pub fn new() -> Self {
        Trie::default()
    }

    pub fn insert(&mut self, word: &str) {
        word.chars()
            .fold(self, |node, c| node.children.entry(c).or_default())
            .is_leaf = true;
    }

    fn get(&self, word: &str) -> Option<&Trie> {
        word.chars().try_fold(self, |node, c| node.children.get(&c))
    }

    fn search(&self, word: &str) -> bool {
        self.get(word).map_or(false, |node| node.is_leaf)
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.get(prefix).is_some()
    }
    pub fn search_prefix(&self, prefix: &str) -> Option<Vec<String>> {
        // 1. 快速定位到前缀所在的节点
        // 如果前缀本身都不存在，直接返回空 Vec，避免后续分配
        let start_node = match self.get(prefix) {
            Some(node) => node,
            None => return None,
        };
        // 2. 准备结果容器
        let mut results = Vec::new();

        // 3. 准备缓冲区 (Buffer)
        // 预分配一定的容量可以进一步减少扩容开销，这里初始化为 prefix
        let mut buffer = String::with_capacity(prefix.len() + 10);
        buffer.push_str(prefix);

        // 4. 开始深度优先搜索 (DFS)
        self.dfs_collect(start_node, &mut buffer, &mut results);

        Some(results)
    }

    // 内部辅助函数：深度优先遍历
    fn dfs_collect(&self, node: &Trie, buffer: &mut String, results: &mut Vec<String>) {
        // 如果当前节点是单词结尾，将 buffer 当前的内容克隆一份存入结果
        if node.is_leaf {
            results.push(buffer.clone());
        }

        // 遍历所有子节点
        for (char, child_node) in &node.children {
            // A. 前进：压入字符
            buffer.push(*char);

            // B. 递归
            self.dfs_collect(child_node, buffer, results);

            // C. 回溯：弹出字符 (还原现场)
            // 这是高性能的关键，复用了 buffer，没有产生新字符串
            buffer.pop();
        }
    }
}

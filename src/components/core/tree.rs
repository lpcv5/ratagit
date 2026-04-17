use std::collections::HashMap;

/// 文件状态标记
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum GitFileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Unmodified,
}

impl GitFileStatus {
    pub fn display_label(&self) -> &'static str {
        match self {
            GitFileStatus::Added => "A",
            GitFileStatus::Modified => "M",
            GitFileStatus::Deleted => "D",
            GitFileStatus::Renamed => "R",
            GitFileStatus::Untracked => "??",
            GitFileStatus::Unmodified => " ",
        }
    }

    #[allow(dead_code)]
    pub fn color_code(&self) -> &'static str {
        match self {
            GitFileStatus::Added => "green",
            GitFileStatus::Modified => "yellow",
            GitFileStatus::Deleted => "red",
            GitFileStatus::Renamed => "cyan",
            GitFileStatus::Untracked => "darkgray",
            GitFileStatus::Unmodified => "reset",
        }
    }
}

/// 树节点
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
    pub status: Option<GitFileStatus>,
    pub is_staged: bool, // 文件是否被 staged
}

impl TreeNode {
    pub fn new(
        path: String,
        name: String,
        is_dir: bool,
        depth: usize,
        status: Option<GitFileStatus>,
    ) -> Self {
        Self {
            path,
            name,
            is_dir,
            is_expanded: true, // 默认展开目录
            depth,
            status,
            is_staged: false,
        }
    }

    #[allow(dead_code)] // Used by tree_component.toggle_node
    pub fn toggle_expanded(&mut self) {
        if self.is_dir {
            self.is_expanded = !self.is_expanded;
        }
    }
}

/// 从扁平路径列表构建树形结构
/// paths: 文件路径列表（使用 / 分隔）
/// status_map: 路径到状态的映射（可选）
pub fn build_tree_from_paths(
    paths: &[String],
    status_map: Option<&HashMap<String, GitFileStatus>>,
) -> Vec<TreeNode> {
    if paths.is_empty() {
        return vec![];
    }

    // 收集所有唯一的目录路径及其深度
    let mut dir_map: HashMap<String, usize> = HashMap::new();
    let mut untracked_dirs: HashMap<String, GitFileStatus> = HashMap::new();

    for path in paths {
        // 处理以 / 结尾的路径（未跟踪的目录）
        let is_dir_path = path.ends_with('/');
        let clean_path = path.trim_end_matches('/');

        if is_dir_path {
            // 这是一个未跟踪的目录，记录它
            let depth = clean_path.split('/').count() - 1;
            dir_map.insert(clean_path.to_string(), depth);
            if let Some(status) = status_map.and_then(|m| m.get(path).copied()) {
                untracked_dirs.insert(clean_path.to_string(), status);
            }
        }

        let parts: Vec<&str> = clean_path.split('/').collect();
        // 对于 src/app/cache.rs，parts = ["src", "app", "cache.rs"]
        // 目录有: "src" (depth 0), "src/app" (depth 1)
        for i in 1..parts.len() {
            let dir_path = parts[..i].join("/");
            dir_map.entry(dir_path).or_insert(i - 1);
        }
    }

    let mut nodes: Vec<TreeNode> = Vec::new();

    // 添加目录节点
    for (dir_path, depth) in &dir_map {
        let name = dir_path
            .split('/')
            .next_back()
            .unwrap_or(dir_path)
            .to_string();
        let status = untracked_dirs.get(dir_path).copied();
        nodes.push(TreeNode::new(dir_path.clone(), name, true, *depth, status));
    }

    // 添加文件节点（跳过以 / 结尾的目录路径）
    for path in paths {
        if path.ends_with('/') {
            continue; // 跳过目录路径，已经在上面处理过了
        }
        let depth = path.split('/').count() - 1;
        let name = path.split('/').next_back().unwrap_or(path).to_string();
        let status = status_map.and_then(|m| m.get(path).copied());
        nodes.push(TreeNode::new(path.clone(), name, false, depth, status));
    }

    // 关键修复：按路径字典序排序，这样父子目录会正确相邻
    // 例如: "src" < "src/app" < "src/app/cache.rs"
    nodes.sort_by(|a, b| {
        // 目录优先于同目录下的文件
        if a.path == b.path {
            return a.is_dir.cmp(&b.is_dir);
        }
        a.path.cmp(&b.path)
    });

    nodes
}

/// 根据展开状态过滤可见节点
pub fn get_visible_nodes(nodes: &[TreeNode]) -> Vec<&TreeNode> {
    let mut visible = Vec::new();
    let mut skip_depth: Option<usize> = None;

    for node in nodes {
        // 如果当前深度大于等于要跳过的深度，说明是被折叠目录的子节点
        if let Some(skip_depth_val) = skip_depth {
            if node.depth >= skip_depth_val {
                continue;
            } else {
                skip_depth = None;
            }
        }

        visible.push(node);

        // 如果是折叠的目录，设置跳过深度
        if node.is_dir && !node.is_expanded {
            skip_depth = Some(node.depth + 1);
        }
    }

    visible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_tree() {
        let paths = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
        let nodes = build_tree_from_paths(&paths, None);

        // 应该有一个目录节点和两个文件节点
        assert_eq!(nodes.len(), 3);

        // 目录节点
        let dir = nodes.iter().find(|n| n.is_dir).unwrap();
        assert_eq!(dir.name, "src");
        assert_eq!(dir.depth, 0);

        // 文件节点
        let files: Vec<_> = nodes.iter().filter(|n| !n.is_dir).collect();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_visible_nodes_respect_expansion() {
        let paths = vec![
            "src/main.rs".to_string(),
            "src/components/tree.rs".to_string(),
            "README.md".to_string(),
        ];
        let nodes = build_tree_from_paths(&paths, None);

        // 默认所有目录展开
        let visible = get_visible_nodes(&nodes);
        // 应该包含所有节点（因为目录默认展开）
        assert!(visible.iter().any(|n| n.name == "src" && n.is_dir));
        assert!(visible.iter().any(|n| n.name == "README.md"));
        assert!(visible.iter().any(|n| n.name == "main.rs" && !n.is_dir));
    }

    #[test]
    fn test_nested_directories() {
        let paths = vec![
            "src/app/cache.rs".to_string(),
            "src/app/components.rs".to_string(),
            "src/backend/commands.rs".to_string(),
            "src/components/core/tree.rs".to_string(),
        ];
        let nodes = build_tree_from_paths(&paths, None);

        // 应该有目录: src(0), src/app(1), src/backend(1), src/components(1), src/components/core(2)
        let dirs: Vec<_> = nodes.iter().filter(|n| n.is_dir).collect();
        assert_eq!(dirs.len(), 5);

        // 验证排序：src < src/app < src/backend < src/components < src/components/core
        assert_eq!(dirs[0].path, "src");
        assert_eq!(dirs[0].depth, 0);
        assert_eq!(dirs[1].path, "src/app");
        assert_eq!(dirs[1].depth, 1);
        assert_eq!(dirs[2].path, "src/backend");
        assert_eq!(dirs[2].depth, 1);

        // 验证可见节点（默认展开，所以所有节点都可见）
        let visible = get_visible_nodes(&nodes);
        assert_eq!(visible.len(), nodes.len()); // 所有节点可见
    }
}

/// `op`
/// 定义了如何将一个字符串转化为另一个字符串的的三种原子操作
#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum Operation {
    /// 保持 - 将 base 字符串游标位置后侧的字符串拷贝到 buffer 中，并将 base 字符串游标向右移动相应长度
    Retain(usize),
    /// 插入 - 向 buffer 中插入字符串，且 base 字符串的游标保持不变
    Insert(String),
    /// 删除 - 移动游标在 base 字符串中，向右移动相应长度，不操作 buffer
    Delete(usize),
}

impl ToString for Operation {
    fn to_string(&self) -> String {
        match self {
            &Self::Retain(n) => format!("retain({})", n),
            Self::Insert(str) => format!("insert(\"{}\")", str.replace('"', "\\\"")),
            &Self::Delete(n) => format!("delete({})", n),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::Operation;

    #[test]
    fn it_works() {
        assert_eq!("retain(1)", Operation::Retain(1).to_string());
        assert_eq!(
            "insert(\"abc\")",
            Operation::Insert("abc".to_string()).to_string()
        );
        assert_eq!(
            "insert(\"abc\\\"\")",
            Operation::Insert("abc\"".to_string()).to_string()
        );
    }
}

use super::error::OperationError;
use super::operation::Operation;
use std::str::Chars;

/// `ops`
/// 本质上是 `[op]` 类型， 定义了如何将一个字符串转换为另一个字符串的 `op` 序列。
/// 注意：当 `baseLength == base.len()` 时，说明虚拟游标移动到该文本的尾部，由于 原子操作 `Operation` 定义的操作，
/// 游标只能向后移动，所以此时，若需要进行 `Retain` 或者 `Delete`，则需要创建一个新的 `TextOperation`
///
/// # Example
/// 针对字符串 `abc`，在 用户删除了 b，并在 c 会后插入了 d。
/// ```
/// use ot_rs::core::TextOperation;
/// let mut ops = TextOperation::new();
/// ops.retain(1).delete(1).retain(1).insert("d");
/// let base = "abc";
/// let after = "acd";
/// assert_eq!(after, ops.apply(base).unwrap());
/// ```
#[derive(Debug)]
pub struct TextOperation {
    /// 原子操作
    ops: Vec<Operation>,
    /// Retain、 Delete 的长度
    /// 在 apply(base) -> after 时，等于 len(base)
    base_length: usize,
    /// Retain、Insert 的长度
    /// 在 apply(base) -> after 时，等于 len(after)
    after_length: usize,
}

impl PartialEq for TextOperation {
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let mut ops1 = TextOperation::new();
    /// let mut ops2 = TextOperation::new();
    /// assert!(ops1 == ops2);
    /// ops1.retain(1).delete(1).retain(1).insert("d");
    /// assert!(ops1 != ops2);
    /// ops2.retain(1).delete(1).retain(1).insert("d");
    /// assert!(ops1 == ops2);
    /// ops2.insert("1");
    /// assert!(ops1 != ops2);
    /// ```
    fn eq(&self, other: &Self) -> bool {
        if self.base_length != other.base_length {
            return false;
        }
        if self.after_length != other.after_length {
            return false;
        }
        if self.ops.len() != other.ops.len() {
            return false;
        }
        return self
            .ops
            .iter()
            .zip(&other.ops)
            .filter(|&(a, b)| *a != *b)
            .count()
            == 0;
    }
}

impl ToString for TextOperation {
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let mut ops = TextOperation::new();
    /// assert_eq!("(0->0){}", ops.to_string());
    /// ops.retain(1).delete(1).retain(1).insert("de\"");
    /// assert_eq!(
    ///     "(3->5){retain(1).delete(1).retain(1).insert(\"de\\\"\")}",
    ///     ops.to_string()
    /// );
    /// ```
    fn to_string(&self) -> String {
        format!("({}->{}){{", self.base_length, self.after_length).to_string()
            + &self
                .ops
                .iter()
                .map(Operation::to_string)
                .collect::<Vec<String>>()
                .join(".")
            + "}"
    }
}

impl TextOperation {
    // === 构造函数 ===
    /// 构造函数，创建一个无操作的 TextOperation
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let mut ops = TextOperation::new();
    /// assert_eq!("(0->0){}", ops.to_string());
    /// ```
    pub fn new() -> TextOperation {
        return TextOperation {
            ops: vec![],
            base_length: 0,
            after_length: 0,
        };
    }

    // === 3 个 操作函数（接收 `&mut self`） ===

    /// 跳过给定数量的字符
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let mut ops = TextOperation::new();
    /// ops.retain(1);
    /// assert_eq!("(1->1){retain(1)}", ops.to_string());
    /// ops.retain(1);
    /// assert_eq!("(2->2){retain(2)}", ops.to_string());
    ///
    pub fn retain(&mut self, n: usize) -> &mut TextOperation {
        if n == 0 {
            return self;
        }
        self.base_length += n;
        self.after_length += n;

        // R(x),R(y) -> R(x+y)
        if let Some(Operation::Retain(last_n)) = self.ops.last_mut() {
            *last_n += n;
        } else {
            self.ops.push(Operation::Retain(n))
        }
        return self;
    }

    /// 在当前位置插入一个字符串
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let mut ops = TextOperation::new();
    /// ops.insert("a");
    /// assert_eq!("(0->1){insert(\"a\")}", ops.to_string());
    /// ops.insert("b");
    /// // 两次连续的插入将合并
    /// assert_eq!("(0->2){insert(\"ab\")}", ops.to_string());
    /// ops.delete(1);
    /// assert_eq!("(1->2){insert(\"ab\").delete(1)}", ops.to_string());
    /// // I,D + I 将加入的 I 合并到前面的插入
    /// ops.insert("c");
    /// assert_eq!("(1->3){insert(\"abc\").delete(1)}", ops.to_string());
    /// ops.retain(1).delete(1);
    /// assert_eq!(
    ///     "(3->4){insert(\"abc\").delete(1).retain(1).delete(1)}",
    ///     ops.to_string()
    /// );
    /// // D + I 将变为 I,D
    /// ops.insert("d");
    /// assert_eq!(
    ///     "(3->5){insert(\"abc\").delete(1).retain(1).insert(\"d\").delete(1)}",
    ///     ops.to_string()
    /// );
    /// ```
    pub fn insert<T: Into<String>>(&mut self, str: T) -> &mut TextOperation {
        let str = str.into();
        if str == "".to_string() {
            return self;
        }
        self.after_length += str.chars().count();
        match self.ops.split_last_mut() {
            // 合并 I(x),I(y) -> I(x+y)
            Some((Operation::Insert(last_str), _)) => last_str.push_str(str.as_str()),
            Some((Operation::Delete(_), op_heads)) => {
                // 始终保持 insert 在 delete 前面
                match op_heads.last_mut() {
                    // 合并 I(s),D(x),I(y) -> I(s+y),D(x)
                    Some(Operation::Insert(last_str)) => last_str.push_str(str.as_str()),
                    // D(x),I(y) -> I(y),D(x)
                    // 参考实现没有 bug，第一步 `ops[ops.length] = ops[ops.length-1]` 相当于插入了一个元素 😂，本质上就是上面的说明
                    // https://github.com/Operational-Transformation/ot.js/blob/e9a3a0e214dd6c001e25515274bae0842a8415f2/lib/text-operation.js#L102
                    _ => {
                        let last_delete = self.ops.last().unwrap().clone();
                        *self.ops.last_mut().unwrap() = Operation::Insert(str);
                        self.ops.push(last_delete);
                    }
                }
            }
            _ => self.ops.push(Operation::Insert(str)),
        }
        return self;
    }

    /// 删除当前位置的字符串
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let mut ops = TextOperation::new();
    /// ops.delete(1);
    /// assert_eq!("(1->0){delete(1)}", ops.to_string());
    /// ops.delete(2);
    /// assert_eq!("(3->0){delete(3)}", ops.to_string());
    /// ```
    pub fn delete(&mut self, n: usize) -> &mut TextOperation {
        if n == 0 {
            return self;
        }
        self.base_length += n;

        // D(x),D(y) -> D(x+y)
        if let Some(Operation::Delete(last_n)) = self.ops.last_mut() {
            *last_n += n;
        } else {
            self.ops.push(Operation::Delete(n))
        }
        return self;
    }

    /// 测试该操作 apply 后是否不产生影响
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let mut ops = TextOperation::new();
    /// assert!(ops.is_noop());
    /// ops.retain(10);
    /// assert!(ops.is_noop());
    /// ```
    pub fn is_noop(&self) -> bool {
        match self.ops.len() {
            0 => true,
            1 => match self.ops.first() {
                Some(&Operation::Retain(_)) => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// 将 操作 apply 应用到 base 字符串中，并返回一个新字符串；
    /// 如果输入的字符串和操作之间不匹配，抛出一个错误。
    /// # Example
    /// ```
    /// use ot_rs::core::{OperationError, TextOperation};
    /// // 正常情况
    /// let mut ops = TextOperation::new();
    /// ops.retain(1).delete(1).retain(1).insert("d");
    /// let base = "abc";
    /// let after = "acd";
    /// assert_eq!(after, ops.apply(base).unwrap());
    /// // 异常情况
    /// let mut ops = TextOperation::new();
    /// assert_eq!(
    ///     OperationError::OperationApplyStringNotCompatible,
    ///     ops.insert("a").apply("---").unwrap_err()
    /// );
    /// ```
    pub fn apply<T: Into<String>>(&self, base: T) -> Result<String, OperationError> {
        let base = base.into();
        let base_len = base.chars().count();
        if base_len != self.base_length {
            return Err(OperationError::OperationApplyStringNotCompatible);
        }

        let base_chars = &mut base.chars(); // 这是一个迭代器，不能使用切片语法，因为字符串是 utf8
        let mut buffer: Vec<String> = Vec::with_capacity(self.ops.len());
        let mut cursor = 0usize;
        for op in &self.ops {
            match op {
                &Operation::Retain(n) => {
                    if cursor + n > base_len {
                        return Err(OperationError::OperationMoreLeftString);
                    }
                    // 遍历迭代器返回 base 前 n 个字符
                    buffer.push(chars_take(base_chars, n));
                    cursor += n // 游标移动
                }
                Operation::Insert(v) => buffer.push(v.clone()),
                &Operation::Delete(n) => {
                    if cursor + n > base_len {
                        return Err(OperationError::OperationMoreLeftString);
                    }
                    cursor += n;
                    // 遍历迭代器，skip 字符
                    chars_skip(base_chars, n);
                }
            }
        }
        // 不可能发生
        // if cursor != base_len {
        //     return Err(OperationError::OperationNotCoverWholeString);
        // }
        return Ok(buffer.join(""));
    }

    /// 生成 该 Operation 的 逆操作，即求 ops' 且满足 `apply(apply(s, ops), ops') = s`。可以用来实现 undo
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let base = "abc";
    ///
    /// let mut ops = TextOperation::new();
    /// ops.retain(1).delete(1).retain(1).insert("d");
    /// assert_eq!(
    ///     base,
    ///     ops.invert(base)
    ///         .unwrap()
    ///         .apply(ops.apply(base).unwrap())
    ///         .unwrap()
    /// );
    /// ```
    pub fn invert<T: Into<String>>(&self, base: T) -> Result<TextOperation, OperationError> {
        let base = base.into();
        let base_len = base.chars().count();
        if base_len != self.base_length {
            return Err(OperationError::OperationApplyStringNotCompatible);
        }

        let base_chars = &mut base.chars(); // 这是一个迭代器，不能使用切片语法，因为字符串是 utf8
        let mut cursor = 0usize;
        let mut inverse = TextOperation::new();
        // abe
        // R1, D1, Icd, D1,
        // acd
        // R1, Ib, D2, Ie
        // abe
        for op in &self.ops {
            match op {
                &Operation::Retain(n) => {
                    if cursor + n > base_len {
                        return Err(OperationError::OperationMoreLeftString);
                    }
                    inverse.retain(n);
                    cursor += n;
                    chars_skip(base_chars, n);
                }
                Operation::Insert(str) => {
                    inverse.delete(str.chars().count());
                }
                &Operation::Delete(n) => {
                    if cursor + n > base_len {
                        return Err(OperationError::OperationMoreLeftString);
                    }
                    inverse.insert(chars_take(base_chars, n));
                    cursor += n;
                }
            }
        }
        // 不可能发生
        // if cursor != base_len {
        //     return Err(OperationError::OperationNotCoverWholeString);
        // }
        return Ok(inverse);
    }

    /// 合并连续的两个 文本操作，满足 `apply(apply(S, A), B) = apply(S, compose(A, B))`
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let base = "abc";
    /// let mut ops1 = TextOperation::new();
    /// ops1.retain(1).insert("123").delete(1).retain(1);
    /// let after1 = ops1.apply(base).unwrap();
    /// assert_eq!("a123c", after1);
    ///
    /// let mut ops2 = TextOperation::new();
    /// ops2.retain(2)
    ///     .insert("$$$")
    ///     .delete(1)
    ///     .retain(1)
    ///     .insert("###")
    ///     .retain(1);
    /// let after2 = ops2.apply(&after1).unwrap();
    ///
    /// assert_eq!("a1$$$3###c", after2);
    /// let compose_ops = ops1.compose(&ops2).unwrap();
    /// assert_eq!(after2, compose_ops.apply(base).unwrap());
    /// ```
    pub fn compose(&self, operation2: &TextOperation) -> Result<TextOperation, OperationError> {
        if self.after_length != operation2.base_length {
            return Err(OperationError::SecondBaseLengthNotEqualFirstAfterLength);
        }

        let mut ops1 = self.ops.split_first();
        let mut ops2 = operation2.ops.split_first();
        let mut tmp: Box<Operation>; // 修复 rust 生命周期检测

        let mut composed = TextOperation::new();
        // 思路大概是：
        // 设置两个游标，同时遍历 ops1，ops2；
        // 每一轮迭代，都相当于重新调用了 compose，是一个递归过程；
        // 定义递归函数 compose(ops1, ops2, ops3) 将 ops1、ops2 合并成 ops3
        // 因此我们只需按照递归的思路，思考初始的状态的9种组合即可
        // 1. compose([R(x), ..ops1], [R(y), ..ops2], [])
        //          x == y: compose(ops1, ops2, [R(x)])
        //          x > y : compose([R(x-y), ..ops1], ops2, [R(y)])
        //          x < y : compose(ops1, [R(y-x), ..ops2], [R(y)])
        // 2. compose([R(x), ..ops1], [I(y), ..ops2], [])
        //                : compose([R(x), ..ops1], ops2, [I(y)])
        // 3. compose([R(x), ..ops1], [D(y), ..ops2], [])
        //          y < x : compose([R(x-y), ..ops1], ops2, [D(y)])
        //           y = x: compose(ops1, ops2, [D(y)])
        //          y > x : compose(ops1, [D(y-x) ,..ops2], [D(x)])
        // 4. compose([I(x), ..ops1], [R(y), ..ops2], [])
        //          ...
        // 在此就不全部枚举了，本质上就 op1、op2 将范围大的那个拆一部分出来，然后继续递归
        // 将以上全部枚举出来后，进行剪枝，并转化为迭代的形式，就可以得到如下的算法
        loop {
            match (ops1, ops2) {
                // None, None
                (None, None) => break,
                // D, _
                (Some((&Operation::Delete(n1), ops_tail1)), _) => {
                    composed.delete(n1);
                    ops1 = ops_tail1.split_first();
                    continue;
                }
                // _, I
                (_, Some((Operation::Insert(s), ops_tail))) => {
                    composed.insert(s.clone());
                    ops2 = ops_tail.split_first();
                    continue;
                }
                // None, _
                (None, _) => return Err(OperationError::ComposeFirstTooShort),
                // _, None
                (_, None) => return Err(OperationError::ComposeFirstTooLong),
                (
                    Some((&Operation::Retain(n1), ops_tail1)),
                    Some((&Operation::Retain(n2), ops_tail2)),
                ) => {
                    if n1 > n2 {
                        composed.retain(n2);
                        tmp = Box::new(Operation::Retain(n1 - n2));
                        ops1 = Some((&tmp, ops_tail1));
                        ops2 = ops_tail2.split_first();
                    } else if n1 == n2 {
                        composed.retain(n1);
                        ops1 = ops_tail1.split_first();
                        ops2 = ops_tail2.split_first();
                    } else {
                        composed.retain(n1);
                        tmp = Box::new(Operation::Retain(n2 - n1));
                        ops2 = Some((&tmp, ops_tail2));
                        ops1 = ops_tail1.split_first();
                    }
                }
                // I, D
                (
                    Some((Operation::Insert(s1), ops_tail1)),
                    Some((&Operation::Delete(n2), ops_tail2)),
                ) => {
                    let l1 = s1.chars().count();
                    if l1 > n2 {
                        tmp = Box::new(Operation::Insert(chars_tail(&mut s1.chars(), n2)));
                        ops1 = Some((&tmp, ops_tail1));
                        ops2 = ops_tail2.split_first();
                    } else if l1 == n2 {
                        ops1 = ops_tail1.split_first();
                        ops2 = ops_tail2.split_first();
                    } else {
                        tmp = Box::new(Operation::Delete(n2 - l1));
                        ops1 = ops_tail1.split_first();
                        ops2 = Some((&tmp, ops_tail2));
                    }
                }
                // I,R
                (
                    Some((Operation::Insert(s1), ops_tail1)),
                    Some((&Operation::Retain(n2), ops_tail2)),
                ) => {
                    let l1 = s1.chars().count();
                    if l1 > n2 {
                        let chars = &mut s1.chars();
                        composed.insert(chars_take(chars, n2));
                        tmp = Box::new(Operation::Insert(chars_take(chars, l1 - n2)));
                        ops1 = Some((&tmp, ops_tail1));
                        ops2 = ops_tail2.split_first();
                    } else if l1 == n2 {
                        composed.insert(s1.clone());
                        ops1 = ops_tail1.split_first();
                        ops2 = ops_tail2.split_first();
                    } else {
                        composed.insert(s1.clone());
                        tmp = Box::new(Operation::Retain(n2 - l1));
                        ops2 = Some((&tmp, ops_tail2));
                        ops1 = ops_tail1.split_first();
                    }
                }
                // R,D
                (
                    Some((&Operation::Retain(n1), ops_tail1)),
                    Some((&Operation::Delete(n2), ops_tail2)),
                ) => {
                    if n1 > n2 {
                        composed.delete(n2);
                        tmp = Box::new(Operation::Retain(n1 - n2));
                        ops1 = Some((&tmp, ops_tail1));
                        ops2 = ops_tail2.split_first();
                    } else if n1 == n2 {
                        composed.delete(n2);
                        ops1 = ops_tail1.split_first();
                        ops2 = ops_tail2.split_first();
                    } else {
                        composed.delete(n1);
                        tmp = Box::new(Operation::Delete(n2 - n1));
                        ops2 = Some((&tmp, ops_tail2));
                        ops1 = ops_tail1.split_first();
                    }
                }
            }
        }
        Ok(composed)
    }

    /// 获取起始游标
    fn first_cursor(&self) -> usize {
        if let Some(&Operation::Retain(n)) = self.ops.first() {
            return n;
        }
        return 0;
    }

    /// 如果当前操作是简单操作，则返回这个简单操作的内容，否者返回 None。
    /// 简单操作指的是：只进行了一次或零次 Insert/Delete 操作
    fn get_simple_operation(&self) -> Option<&Operation> {
        match self.ops.as_slice() {
            // [_] => [0]
            [first] => Some(first),
            // [R, _] => [1]
            [Operation::Retain(_), second] => Some(second),
            // [I|D, R] => [0]
            [first, Operation::Retain(_)] => Some(first),
            // [R, _, R] => [1]
            [Operation::Retain(_), second, Operation::Retain(_)] => Some(second),
            _ => None,
        }
    }

    /// 当使用 ctrl-z 撤消最近的更改时，希望程序不会撤消每一次击键，而是撤消一口气写下的最后一句话或通过按住退格键所做的删除。
    /// 这可以通过在将撤消栈上的操作进行 compose 来实现。 这个方法可以帮助决定是否应该组合两个操作。
    /// 如果操作是 `连续的插入操作` 或 `连续的删除操作`，则返回 true。
    /// 可能希望包括其他因素，例如自上次更改决定以来的时间。
    /// # Example
    /// ```
    /// use ot_rs::core::TextOperation;
    /// let mut ops1: TextOperation;
    /// let mut ops2: TextOperation;
    /// // noop;I / I;noop
    /// ops1 = TextOperation::new();
    /// ops1.retain(3);
    /// ops2 = TextOperation::new();
    /// ops2.retain(1).insert("xxx").retain(2);
    /// assert!(ops1.should_be_composed_with(&ops2));
    /// assert!(ops2.should_be_composed_with(&ops1));
    /// // I;I 正常输入
    /// ops1 = TextOperation::new();
    /// ops1.retain(1).insert("a").retain(2);
    /// ops2 = TextOperation::new();
    /// ops2.retain(2).insert("b").retain(2);
    /// assert!(ops1.should_be_composed_with(&ops2));
    /// ops1.delete(3);
    /// assert!(!ops1.should_be_composed_with(&ops2));
    /// // I;I 插入后光标发生变化
    /// ops1 = TextOperation::new();
    /// ops1.retain(1).insert("b").retain(2);
    /// ops2 = TextOperation::new();
    /// ops2.retain(1).insert("a").retain(3);
    /// assert!(!ops1.should_be_composed_with(&ops2));
    /// // D;D 退格键方式
    /// ops1 = TextOperation::new();
    /// ops1.retain(4).delete(3).retain(10);
    /// ops2 = TextOperation::new();
    /// ops2.retain(2).delete(2).retain(10);
    /// assert!(ops1.should_be_composed_with(&ops2));
    /// // D;D delete键方式
    /// ops2 = TextOperation::new();
    /// ops2.retain(4).delete(7).retain(3);
    /// assert!(ops1.should_be_composed_with(&ops2));
    /// // D;D 不连续的删除
    /// ops2 = TextOperation::new();
    /// ops2.retain(2).delete(9).retain(3);
    /// assert!(!ops1.should_be_composed_with(&ops2));
    /// ```
    pub fn should_be_composed_with(&self, other: &TextOperation) -> bool {
        // 无影响的操作，可以合并
        if self.is_noop() || other.is_noop() {
            return true;
        }
        let (a_first_cursor, b_first_cursor) = (self.first_cursor(), other.first_cursor());
        let (a_sample, b_sample) = (self.get_simple_operation(), other.get_simple_operation());
        // 只要一个是非简单操作，则不可以合并
        if a_sample.is_none() || b_sample.is_none() {
            return false;
        }
        match (a_sample, b_sample, a_first_cursor, b_first_cursor) {
            // I, I - 保证后插入的在之前插入的后方进行插入
            (Some(Operation::Insert(str)), Some(Operation::Insert(_)), _, _) => {
                return str.chars().count() + a_first_cursor == b_first_cursor; // 连续输入两个字符
            }
            // D, D
            (Some(&Operation::Delete(_)), Some(&Operation::Delete(dn2)), _, _) => {
                return b_first_cursor as i64 + dn2 as i64 == a_first_cursor as i64 // 按两下退格的场景
                    || a_first_cursor == b_first_cursor; // 按两下 delete 键的场景
            }
            // 其他情况
            _ => false,
        }
    }

    /// 决定两个操作如果被 invert 是否应该相互组合，即 `should_be_composed_with_inverted(a, b) = should_be_composed_with_inverted(b^{-1}, a^{-1})`
    pub fn should_be_composed_with_inverted(&self, other: &TextOperation) -> bool {
        // 无影响的操作，可以合并
        if self.is_noop() || other.is_noop() {
            return true;
        }
        let (a_first_cursor, b_first_cursor) = (self.first_cursor(), other.first_cursor());
        let (a_sample, b_sample) = (self.get_simple_operation(), other.get_simple_operation());
        // 只要一个是非简单操作，则不可以合并
        if a_sample.is_none() || b_sample.is_none() {
            return false;
        }
        match (a_sample, b_sample, a_first_cursor, b_first_cursor) {
            // I, I - 因为是逆，所以原操作是 Delete
            (Some(Operation::Insert(str)), Some(Operation::Insert(_)), _, _) => {
                return a_first_cursor + str.chars().count() == b_first_cursor
                    || a_first_cursor == b_first_cursor;
            }
            // D, D - 因为是逆，所以原操作是 Insert
            (Some(&Operation::Delete(_)), Some(&Operation::Delete(dn2)), _, _) => {
                return b_first_cursor as i64 - dn2 as i64 == a_first_cursor as i64
            }
            // 其他情况
            _ => false,
        }
    }

    /// 这个函数是 OT 算法的核心。
    /// 转换两个基于同一版本 S 的操作 A 和 B，返回 A' 和 B'，使其满足
    /// `apply(apply(S, A), B') = apply(apply(S, B), A')`。
    pub fn transform(
        &self,
        operation2: &TextOperation,
    ) -> Result<(TextOperation, TextOperation), OperationError> {
        let operation1 = self;
        if operation1.base_length != operation2.base_length {
            return Err(OperationError::TransformBaseDifferent);
        }

        let mut tmp: Box<Operation>; // 修复 rust 生命周期检测
        let (mut operation1prime, mut operation2prime) =
            (TextOperation::new(), TextOperation::new());

        let mut ops1 = self.ops.split_first();
        let mut ops2 = operation2.ops.split_first();
        // 和 compose 方法类似
        // 思路大概是：
        // 设置两个游标，同时遍历 ops1，ops2；
        // 每一轮迭代，都要保证游标，在 S 的位置是一致的
        // 因此我们只需按照递归的思路，思考初始的状态的9种组合即可。
        // 全部枚举出来后，进行剪枝，就可以得到如下的算法
        loop {
            match (ops1, ops2) {
                (None, None) => break,
                // 如下两种情况：只要有一方是 Insert，这一方面方的 Prime 就跳过，量一方的 Prime 就插入
                // (3 种情况) I, _
                (Some((Operation::Insert(str1), tail1)), _) => {
                    operation1prime.insert(str1.clone());
                    operation2prime.retain(str1.chars().count());
                    ops1 = tail1.split_first();
                }
                // (2 种情况) _, I
                (_, Some((Operation::Insert(str2), tail2))) => {
                    operation1prime.retain(str2.chars().count());
                    operation2prime.insert(str2.clone());
                    ops2 = tail2.split_first();
                }
                // 异常：只要有一方完成另一方未完成，则报错
                (None, _) => return Err(OperationError::ComposeFirstTooShort),
                (_, None) => return Err(OperationError::ComposeFirstTooLong),
                // (1 种情况) R, R
                (Some((&Operation::Retain(n1), tail1)), Some((&Operation::Retain(n2), tail2))) => {
                    let min_n = if n1 > n2 {
                        tmp = Box::new(Operation::Retain(n1 - n2));
                        ops1 = Some((&tmp, tail1));
                        ops2 = tail2.split_first();
                        n2
                    } else if n1 == n2 {
                        ops1 = tail1.split_first();
                        ops2 = tail2.split_first();
                        n2
                    } else {
                        tmp = Box::new(Operation::Retain(n2 - n1));
                        ops1 = tail1.split_first();
                        ops2 = Some((&tmp, tail2));
                        n1
                    };
                    operation1prime.retain(min_n);
                    operation2prime.retain(min_n);
                }
                // (1 种情况) D, D
                // 同时删除，我们只需要将删除长的保留后面部分，删除短的直接跳过
                (Some((&Operation::Delete(n1), tail1)), Some((&Operation::Delete(n2), tail2))) => {
                    if n1 > n2 {
                        tmp = Box::new(Operation::Delete(n1 - n2));
                        ops1 = Some((&tmp, tail1));
                        ops2 = tail2.split_first();
                    } else if n1 == n2 {
                        ops1 = tail1.split_first();
                        ops2 = tail2.split_first();
                    } else {
                        tmp = Box::new(Operation::Delete(n2 - n1));
                        ops1 = tail1.split_first();
                        ops2 = Some((&tmp, tail2));
                    }
                }
                // 接下来两种情况是 D,R 和 R,D
                // (1 种情况) D, R
                (Some((&Operation::Delete(n1), tail1)), Some((&Operation::Retain(n2), tail2))) => {
                    let min_n = if n1 > n2 {
                        tmp = Box::new(Operation::Delete(n1 - n2));
                        ops1 = Some((&tmp, tail1));
                        ops2 = tail2.split_first();
                        n2
                    } else if n1 == n2 {
                        ops1 = tail1.split_first();
                        ops2 = tail2.split_first();
                        n2
                    } else {
                        tmp = Box::new(Operation::Retain(n2 - n1));
                        ops1 = tail1.split_first();
                        ops2 = Some((&tmp, tail2));
                        n1
                    };
                    operation1prime.delete(min_n);
                }
                // (1 种情况) R, D
                (Some((&Operation::Retain(n1), tail1)), Some((&Operation::Delete(n2), tail2))) => {
                    let min_n = if n1 > n2 {
                        tmp = Box::new(Operation::Retain(n1 - n2));
                        ops1 = Some((&tmp, tail1));
                        ops2 = tail2.split_first();
                        n2
                    } else if n1 == n2 {
                        ops1 = tail1.split_first();
                        ops2 = tail2.split_first();
                        n2
                    } else {
                        tmp = Box::new(Operation::Delete(n2 - n1));
                        ops1 = tail1.split_first();
                        ops2 = Some((&tmp, tail2));
                        n1
                    };
                    operation2prime.delete(min_n);
                } // _ => return Err(OperationError::TransformNotCompatible),
            }
        }
        return Ok((operation1prime, operation2prime));
    }
}

impl Default for TextOperation {
    fn default() -> Self {
        Self::new()
    }
}

fn chars_take(chars: &mut Chars, n: usize) -> String {
    (0..n).map(|_| chars.next().unwrap()).collect::<String>()
}

fn chars_tail(chars: &mut Chars, skip: usize) -> String {
    chars_skip(chars, skip);
    chars.collect::<String>()
}

fn chars_skip(chars: &mut Chars, n: usize) {
    (0..n).for_each(|_| {
        chars.next().unwrap();
    })
}

#[cfg(test)]
mod tests {

    use crate::core::operation::Operation;

    use super::TextOperation;
    use rand::{self, Rng};

    const CHARSET: [char; 10] = ['a', 'b', 'c', '1', '2', '3', '中', '文', '😄', '😂'];
    // const RAND_TEST_COUNT: usize = 500;
    const RAND_TEST_COUNT: usize = 100;

    fn random_string(n: usize) -> String {
        let mut rng: rand::prelude::ThreadRng = rand::thread_rng();
        (0..n)
            .map(|_| rng.gen_range(0..CHARSET.len()))
            .map(|i| CHARSET[i])
            .collect()
    }

    fn random_operation<T: Into<String>>(base: T) -> TextOperation {
        let base = base.into();
        let mut ops = TextOperation::new();
        let mut rng: rand::prelude::ThreadRng = rand::thread_rng();
        loop {
            let left = base.chars().count() - ops.base_length;
            if left == 0 {
                break;
            }
            let r = rng.gen_range(0.0..1.0);
            let l = rng.gen_range(1..=left);
            if r < 0.2 {
                ops.insert(random_string(l));
            } else if r < 0.4 {
                ops.delete(l);
            } else {
                ops.retain(l);
            }
        }
        if rng.gen_range(0.0..1.0) < 0.3 {
            ops.insert(random_string(10));
        }
        ops
    }

    fn run_n(n: usize, f: fn() -> ()) {
        for _ in 0..n {
            f();
        }
    }

    #[test]
    fn test_apply() {
        run_n(RAND_TEST_COUNT, || {
            let base = random_string(50);
            let ops = random_operation(&base);
            let after = ops.apply(&base).unwrap();
            println!("  {} \n->\n  {} \nby\n  {}", &base, &after, ops.to_string());
            assert_eq!(base.chars().count(), ops.base_length);
            assert_eq!(after.chars().count(), ops.after_length);
        })
    }

    #[test]
    fn test_invert() {
        run_n(RAND_TEST_COUNT, || {
            let base = random_string(50);
            let ops = random_operation(&base);
            assert_eq!(
                base,
                ops.invert(&base)
                    .unwrap()
                    .apply(ops.apply(&base).unwrap())
                    .unwrap()
            );
        })
    }

    #[test]
    fn test_compose() {
        run_n(RAND_TEST_COUNT, || {
            // after2 = apply(apply(base, ops1), ops2)
            let base = random_string(50);
            let ops1 = random_operation(&base);
            let after1 = ops1.apply(&base).unwrap();
            let ops2 = random_operation(&after1);
            let after2 = ops2.apply(&after1).unwrap();
            // ops1 + ops2 = compose_ops;
            let compose_ops = ops1.compose(&ops2).unwrap();
            assert_eq!(after2, compose_ops.apply(&base).unwrap());
        })
    }

    #[test]
    fn test_first_cursor() {
        assert_eq!(0, TextOperation::new().first_cursor());
        assert_eq!(0, TextOperation::new().delete(1).first_cursor());
        assert_eq!(1, TextOperation::new().retain(1).first_cursor());
        assert_eq!(0, TextOperation::new().insert("a").first_cursor());
    }

    #[test]
    fn test_get_simple_operation() {
        assert_eq!(None, TextOperation::new().get_simple_operation());
        assert_eq!(
            &Operation::Delete(1),
            TextOperation::new()
                .delete(1)
                .get_simple_operation()
                .unwrap()
        );
        assert_eq!(
            &Operation::Retain(1),
            TextOperation::new()
                .retain(1)
                .get_simple_operation()
                .unwrap()
        );
        assert_eq!(
            &Operation::Insert("abc".to_string()),
            TextOperation::new()
                .retain(1)
                .insert("abc")
                .retain(1)
                .get_simple_operation()
                .unwrap()
        );
    }

    #[test]
    fn should_be_composed_with_inverted() {
        run_n(RAND_TEST_COUNT, || {
            // invariant: should_be_composed_with_inverted(a, b) = should_be_composed_with_inverted(b^{-1}, a^{-1})
            let base = random_string(50);
            let ops1 = random_operation(&base);
            let ops1_inverted = ops1.invert(&base).unwrap();
            let after1 = ops1.apply(&base).unwrap();

            let ops2 = random_operation(&after1);
            let ops2_inverted = ops2.invert(&after1).unwrap();
            assert_eq!(
                ops1.should_be_composed_with(&ops2),
                ops2_inverted.should_be_composed_with_inverted(&ops1_inverted),
            );
        })
    }

    #[test]
    fn should_transform() {
        // transform(a, b) => ('a, 'b)
        // apply(apply(s, a), a') = apply(apply(s, b), a')
        // compose(a, b') = compose(b, a')
        run_n(RAND_TEST_COUNT, || {
            let base = random_string(50);
            let sa = random_operation(&base);
            let sb = random_operation(&base);
            let (sa_prime, sb_prime) = TextOperation::transform(&sa, &sb).unwrap();
            let ab_prime = sa.compose(&sb_prime).unwrap();
            let ba_prime = sb.compose(&sa_prime).unwrap();
            let sa_sb_prime_after = ab_prime.apply(&base);
            let sb_sa_prime_after = ba_prime.apply(&base);
            assert_eq!(ab_prime, ba_prime);
            assert_eq!(sa_sb_prime_after, sb_sa_prime_after);
        });
    }
}

/// 定义 OT 算法的一些异常
#[derive(Debug, PartialEq, Eq)]
pub enum OperationError {
    /// The operation's base length must be equal to the string's length.
    /// 操作的 base length 必须等于 base 字符串的长度
    OperationApplyStringNotCompatible,
    /// Operation can't retain more characters than are left in the string.
    /// 操作长度不能超过剩余的字符字符串长度
    OperationMoreLeftString,
    // /// The operation didn't operate on the whole string.
    // /// 操作不能覆盖整个字符串
    // OperationNotCoverWholeString,
    /// The base length of the second operation has to be the target length of the first operation
    /// 第二个的 base length 不等于第一个的 target length
    SecondBaseLengthNotEqualFirstAfterLength,
    /// compose operations: first operation is too short.
    /// 组合操作：第一个操作太短
    ComposeFirstTooShort,
    /// compose operations: first operation is too long.
    /// 组合操作：第一个操作太长
    ComposeFirstTooLong,
    // /// This shouldn't happen
    // /// 这种情况不应该发生
    // ComposeShouldNotHappen(String),
    /// Both operations have to have the same base length
    /// 两个操作必须具有相同的 base length
    TransformBaseDifferent,
    /// The two operations aren't compatible
    /// 两个操作并不兼容
    TransformNotCompatible,
}

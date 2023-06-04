pub enum Token {
    Root(RootPathToken),
    Property(PropertyPathToken),
    ArrayIndex(ArrayIndexPathToken),
    ArrayPath(ArrayPathPathToken),
    ArraySlice(ArraySlicePathToken),
    Predicate(PredicatePathToken),
    Function(FunctionPathToken),
    Scan(ScanPathToken),
    Wildcard(WildcardPathToken),
}

pub struct RootPathToken {
    pub root_path_char: char,
}
pub struct PropertyPathToken {}
pub struct ArrayIndexPathToken {}
pub struct ArrayPathPathToken {}
pub struct ArraySlicePathToken {}
pub struct PredicatePathToken {}
pub struct FunctionPathToken {}
pub struct ScanPathToken {}
pub struct WildcardPathToken {}

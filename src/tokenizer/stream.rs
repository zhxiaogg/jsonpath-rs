use std::iter::Peekable;
pub trait PeekableExt<Item> {
    fn drop_while<P>(&mut self, predicate: P)
    where
        P: FnMut(&Item) -> bool;
}

impl<I: Iterator> PeekableExt<I::Item> for Peekable<I> {
    fn drop_while<P>(&mut self, mut predicate: P)
    where
        P: FnMut(&I::Item) -> bool,
    {
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.next();
            } else {
                break;
            }
        }
    }
}

pub fn clone_for_look_ahead(
    stream: &Peekable<impl Iterator<Item = char> + Clone>,
) -> Peekable<impl Iterator<Item = char> + Clone> {
    stream.clone()
}

use std::iter::Peekable;

pub trait PeekExt<T> {
    fn peek_while<F, V>(self, pattern: F) -> V
        where 
            F: Fn(&T) -> bool,
            V: FromIterator<T>;
}

impl<I, T> PeekExt<T> for &mut Peekable<I>
where
    I: Iterator<Item = T>
{
    fn peek_while<F, V>(self, pattern: F) -> V
    where 
        F: Fn(&T) -> bool,
        V: FromIterator<T>
    {
        let mut output = Vec::new();
        while let Some(x) = self.peek() {
            if pattern(&x) {
                // We know code.next() works because we peeked.
                output.push(self.next().unwrap());
            } else {
                break;
            }
        }
        output.into_iter().collect::<V>()
    }
}
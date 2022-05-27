pub fn position_of_sequence<T: Eq>(buf: &[T], seq: &[T]) -> Option<usize> {
    let len = seq.len();
    for i in 0..buf.len() - len {
        if &buf[i..i + len] == seq {
            return Some(i);
        }
    }
    None
}

pub fn last_position_of_sequence<T: Eq>(buf: &[T], seq: &[T]) -> Option<usize> {
    let len = seq.len();
    for i in (0..buf.len() - len).rev() {
        if &buf[i..i + len] == seq {
            return Some(i);
        }
    }
    None
}
